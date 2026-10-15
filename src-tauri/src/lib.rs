use lettre::{message::{header::ContentType, Mailbox, Message}, transport::smtp::authentication::Credentials, SmtpTransport, Transport};
use native_tls::TlsConnector;
use serde::{Deserialize, Serialize};
use std::{fs, net::TcpStream, path::PathBuf};
use tauri::Manager;

const SERVICE: &str = "com.maildesk.app";

#[derive(Serialize)]
struct Folder { name: String, delimiter: Option<String>, attributes: Vec<String> }
#[derive(Serialize)]
struct MailRow { uid: u32, from: String, subject: String, date: String, flags: Vec<String>, size: Option<u32> }
#[derive(Deserialize)]
struct SendRequest { username: String, to: String, cc: Option<String>, subject: String, html: String, from_name: Option<String> }
#[derive(Serialize, Deserialize, Default)]
struct Preferences { theme: Option<String>, page_size: Option<u32>, columns: Option<Vec<String>>, bookmarks: Option<Vec<Bookmark>>, identity: Option<String> }
#[derive(Serialize, Deserialize, Clone)]
struct Bookmark { title: String, url: String, pinned: bool }

fn secret(username: &str) -> Result<String, String> {
    keyring::Entry::new(SERVICE, username).map_err(err)?.get_password().map_err(err)
}
fn err<E: std::fmt::Display>(e: E) -> String { e.to_string() }
fn imap_session(username: &str) -> Result<imap::Session<native_tls::TlsStream<TcpStream>>, String> {
    let password = secret(username)?;
    let tls = TlsConnector::builder().build().map_err(err)?;
    let client = imap::connect(("imap.gmail.com", 993), "imap.gmail.com", &tls).map_err(err)?;
    client.login(username, &password).map_err(|(e, _client)| err(e))
}

#[tauri::command]
fn save_credentials(username: String, app_password: String) -> Result<(), String> {
    if !username.contains('@') || app_password.trim().is_empty() { return Err("Enter a valid email address and App Password".into()); }
    let entry = keyring::Entry::new(SERVICE, &username).map_err(err)?;
    entry.set_password(app_password.trim()).map_err(err)
}
#[tauri::command]
fn delete_credentials(username: String) -> Result<(), String> {
    keyring::Entry::new(SERVICE, &username).map_err(err)?.delete_credential().map_err(err)
}
#[tauri::command]
fn test_connection(username: String) -> Result<String, String> {
    let mut session = imap_session(&username)?;
    let caps = session.capabilities().map_err(err)?;
    let _ = session.logout();
    Ok(format!("Connected to Gmail IMAP. Server capabilities: {}", caps.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", ")))
}
#[tauri::command]
fn list_folders(username: String) -> Result<Vec<Folder>, String> {
    let mut session = imap_session(&username)?;
    let result = session.list(Some(""), Some("*")).map_err(err)?.iter().map(|f| Folder {
        name: f.name().to_string(), delimiter: f.delimiter().map(|d| d.to_string()),
        attributes: f.attributes().iter().map(|a| format!("{:?}", a)).collect()
    }).collect();
    let _ = session.logout();
    Ok(result)
}
#[tauri::command]
fn list_messages(username: String, folder: String, page: u32, page_size: u32) -> Result<Vec<MailRow>, String> {
    let mut session = imap_session(&username)?;
    session.select(&folder).map_err(err)?;
    // This initial implementation fetches a bounded tail of UIDs. Full server-side SORT/search,
    // robust Gmail label mapping and RFC-compliant MIME parsing are production milestones.
    let all = session.search("ALL").map_err(err)?;
    let mut ids: Vec<u32> = all.iter().copied().collect();
    ids.sort_unstable(); ids.reverse();
    let size = page_size.clamp(1, 100) as usize;
    let start = page.saturating_sub(1) as usize * size;
    let chosen: Vec<u32> = ids.into_iter().skip(start).take(size).collect();
    let mut rows = Vec::new();
    for uid in chosen {
        let fetched = session.fetch(uid.to_string(), "(UID FLAGS RFC822.SIZE ENVELOPE INTERNALDATE)").map_err(err)?;
        if let Some(m) = fetched.iter().next() {
            let env = m.envelope();
            let from = env.and_then(|e| e.from.as_ref()).and_then(|v| v.first()).map(|a| {
                let mailbox = a.mailbox.as_ref().map(|b| String::from_utf8_lossy(b).to_string()).unwrap_or_default();
                let host = a.host.as_ref().map(|b| String::from_utf8_lossy(b).to_string()).unwrap_or_default();
                let name = a.name.as_ref().map(|b| String::from_utf8_lossy(b).to_string()).unwrap_or_default();
                if name.is_empty() { format!("{}@{}", mailbox, host) } else { name }
            }).unwrap_or_default();
            let subject = env.and_then(|e| e.subject.as_ref()).map(|b| String::from_utf8_lossy(b).to_string()).unwrap_or_default();
            let date = env.and_then(|e| e.date.as_ref()).map(|b| String::from_utf8_lossy(b).to_string()).unwrap_or_default();
            rows.push(MailRow { uid, from, subject, date, flags: m.flags().iter().map(|f| format!("{:?}", f)).collect(), size: m.size() });
        }
    }
    let _ = session.logout();
    Ok(rows)
}
#[tauri::command]
fn send_email(req: SendRequest) -> Result<String, String> {
    let password = secret(&req.username)?;
    let from: Mailbox = req.from_name.as_deref().unwrap_or(&req.username).parse().map_err(err)?;
    let to: Mailbox = req.to.parse().map_err(err)?;
    let mut builder = Message::builder().from(from).to(to).subject(&req.subject).header(ContentType::TEXT_HTML);
    if let Some(cc) = req.cc.as_deref().filter(|s| !s.trim().is_empty()) { builder = builder.cc(cc.parse().map_err(err)?); }
    let message = builder.body(req.html).map_err(err)?;
    let creds = Credentials::new(req.username.clone(), password);
    let mailer = SmtpTransport::relay("smtp.gmail.com").map_err(err)?.port(465).credentials(creds).build();
    mailer.send(&message).map_err(err)?;
    Ok("Message accepted by Gmail SMTP".into())
}
fn preferences_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(err)?;
    fs::create_dir_all(&dir).map_err(err)?;
    Ok(dir.join("preferences.json"))
}
#[tauri::command]
fn load_preferences(app: tauri::AppHandle) -> Result<Preferences, String> {
    let path = preferences_path(&app)?;
    if !path.exists() { return Ok(Preferences::default()); }
    serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)
}
#[tauri::command]
fn save_preferences(app: tauri::AppHandle, preferences: Preferences) -> Result<(), String> {
    let path = preferences_path(&app)?;
    let bytes = serde_json::to_vec_pretty(&preferences).map_err(err)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes).map_err(err)?;
    fs::rename(tmp, path).map_err(err)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![save_credentials, delete_credentials, test_connection, list_folders, list_messages, send_email, load_preferences, save_preferences])
        .run(tauri::generate_context!())
        .expect("error while running MailDesk");
}

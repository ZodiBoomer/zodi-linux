use adw::prelude::*;
use gtk::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;

fn api_base() -> String {
    std::env::var("ZODI_API_URL")
        .unwrap_or_else(|_| "https://zodi-ai-core.vercel.app".to_string())
        .trim_end_matches('/')
        .to_string()
}

fn request_json(method: reqwest::Method, path: &str, token: Option<&str>, body: Option<Value>) -> Result<Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(70))
        .build()
        .map_err(|e| e.to_string())?;
    let mut request = client.request(method, format!("{}{}", api_base(), path));
    if let Some(value) = token {
        request = request.bearer_auth(value);
    }
    if let Some(payload) = body {
        request = request.json(&payload);
    }
    let response = request.send().map_err(|e| format!("Connexion impossible : {e}"))?;
    let status = response.status();
    let data: Value = response.json().unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        return Err(data["error"].as_str().unwrap_or("Erreur du serveur Zodi").to_string());
    }
    Ok(data)
}

#[derive(Clone)]
enum UiEvent {
    Authenticated { token: String, email: String },
    ConversationReady { id: String, transcript: String },
    ChatReply(Result<String, String>),
    LoggedOut,
    Error(String),
}

struct UiState {
    stack: gtk::Stack,
    login_status: gtk::Label,
    chat_status: gtk::Label,
    chat_buffer: gtk::TextBuffer,
    chat_input: gtk::Entry,
    session: Option<String>,
    conversation_id: Option<String>,
    sender: async_channel::Sender<UiEvent>,
}

fn load_conversation(token: String, sender: async_channel::Sender<UiEvent>) {
    thread::spawn(move || {
        let result = (|| -> Result<(String, String), String> {
            let list = request_json(reqwest::Method::GET, "/api/conversations", Some(&token), None)?;
            let conversations = list["conversations"].as_array().cloned().unwrap_or_default();
            let id = if let Some(first) = conversations.first() {
                first["id"].as_str().unwrap_or_default().to_string()
            } else {
                let created = request_json(reqwest::Method::POST, "/api/conversations", Some(&token), Some(json!({})))?;
                created["conversation"]["id"].as_str().unwrap_or_default().to_string()
            };
            if id.is_empty() { return Err("Le serveur n'a pas retourné d'identifiant de conversation.".into()); }
            let detail = request_json(reqwest::Method::GET, &format!("/api/conversations/{id}"), Some(&token), None)?;
            let messages = detail["conversation"]["messages"].as_array().cloned().unwrap_or_default();
            let transcript = messages.iter().map(|item| {
                let role = if item["role"].as_str() == Some("user") { "Toi" } else { "Zodi" };
                format!("{role} : {}\n\n", item["content"].as_str().unwrap_or(""))
            }).collect::<String>();
            Ok((id, transcript))
        })();
        match result {
            Ok((id, transcript)) => { let _ = sender.send_blocking(UiEvent::ConversationReady { id, transcript }); }
            Err(error) => { let _ = sender.send_blocking(UiEvent::Error(error)); }
        }
    });
}

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Zodi AI")
        .default_width(980)
        .default_height(700)
        .build();

    let stack = gtk::Stack::builder().hexpand(true).vexpand(true).transition_type(gtk::StackTransitionType::Crossfade).build();
    let (sender, receiver) = async_channel::unbounded::<UiEvent>();

    let login_page = gtk::Box::new(gtk::Orientation::Vertical, 14);
    login_page.set_halign(gtk::Align::Center);
    login_page.set_valign(gtk::Align::Center);
    login_page.set_width_request(360);
    let logo = gtk::Label::builder().label("✦").css_classes(["title-1"]).build();
    let title = gtk::Label::builder().label("Bienvenue sur Zodi AI").css_classes(["title-1"]).build();
    let subtitle = gtk::Label::new(Some("Connecte-toi avec ton compte Zodi"));
    let email = gtk::Entry::builder().placeholder_text("Adresse e-mail").input_purpose(gtk::InputPurpose::Email).build();
    let password = gtk::PasswordEntry::builder().placeholder_text("Mot de passe").show_peek_icon(true).build();
    let login_button = gtk::Button::with_label("Se connecter");
    login_button.add_css_class("suggested-action");
    let register_button = gtk::Button::with_label("Créer un compte");
    let login_status = gtk::Label::new(Some(""));
    login_status.add_css_class("dim-label");
    login_page.append(&logo);
    login_page.append(&title);
    login_page.append(&subtitle);
    login_page.append(&email);
    login_page.append(&password);
    login_page.append(&login_button);
    login_page.append(&register_button);
    login_page.append(&login_status);
    stack.add_named(&login_page, Some("login"));

    let chat_page = gtk::Box::new(gtk::Orientation::Vertical, 10);
    chat_page.set_margin_top(18);
    chat_page.set_margin_bottom(18);
    chat_page.set_margin_start(18);
    chat_page.set_margin_end(18);
    let chat_header = adw::HeaderBar::new();
    let chat_title = gtk::Label::builder().label("Zodi AI").css_classes(["title-2"]).build();
    chat_header.set_title_widget(Some(&chat_title));
    let new_chat = gtk::Button::with_label("Nouvelle discussion");
    let logout = gtk::Button::with_label("Déconnexion");
    chat_header.pack_start(&new_chat);
    chat_header.pack_end(&logout);
    let chat_status = gtk::Label::new(Some("Connexion…"));
    chat_status.set_halign(gtk::Align::Start);
    chat_status.add_css_class("dim-label");
    let chat_buffer = gtk::TextBuffer::new(None);
    let chat_view = gtk::TextView::with_buffer(&chat_buffer);
    chat_view.set_editable(false);
    chat_view.set_cursor_visible(false);
    chat_view.set_wrap_mode(gtk::WrapMode::WordChar);
    chat_view.set_left_margin(12);
    chat_view.set_right_margin(12);
    let scroll = gtk::ScrolledWindow::builder().vexpand(true).child(&chat_view).build();
    let composer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let chat_input = gtk::Entry::builder().hexpand(true).placeholder_text("Écris un message à Zodi…").build();
    let send_button = gtk::Button::with_label("Envoyer");
    send_button.add_css_class("suggested-action");
    composer.append(&chat_input);
    composer.append(&send_button);
    chat_page.append(&chat_header);
    chat_page.append(&chat_status);
    chat_page.append(&scroll);
    chat_page.append(&composer);
    stack.add_named(&chat_page, Some("chat"));

    let state = Rc::new(RefCell::new(UiState {
        stack: stack.clone(),
        login_status: login_status.clone(),
        chat_status: chat_status.clone(),
        chat_buffer: chat_buffer.clone(),
        chat_input: chat_input.clone(),
        session: None,
        conversation_id: None,
        sender: sender.clone()
    }));

    let login_state = state.clone();
    let login_email = email.clone();
    let login_password = password.clone();
    login_button.connect_clicked(move |_| {
        let address = login_email.text().to_string();
        let secret = login_password.text().to_string();
        let tx = login_state.borrow().sender.clone();
        login_state.borrow().login_status.set_text("Connexion en cours…");
        thread::spawn(move || {
            let result = request_json(reqwest::Method::POST, "/api/mobile/auth/login", None, Some(json!({"email": address, "password": secret})));
            match result {
                Ok(data) => {
                    let token = data["token"].as_str().unwrap_or_default().to_string();
                    let email = data["user"]["email"].as_str().unwrap_or_default().to_string();
                    if token.is_empty() { let _ = tx.send_blocking(UiEvent::Error("Jeton de session manquant.".into())); }
                    else { let _ = tx.send_blocking(UiEvent::Authenticated { token, email }); }
                }
                Err(error) => { let _ = tx.send_blocking(UiEvent::Error(error)); }
            }
        });
    });

    let register_state = state.clone();
    let register_email = email.clone();
    let register_password = password.clone();
    register_button.connect_clicked(move |_| {
        let address = register_email.text().to_string();
        let secret = register_password.text().to_string();
        let tx = register_state.borrow().sender.clone();
        register_state.borrow().login_status.set_text("Création du compte…");
        thread::spawn(move || {
            let result = request_json(reqwest::Method::POST, "/api/mobile/auth/register", None, Some(json!({"email": address, "password": secret})));
            match result {
                Ok(data) => {
                    let token = data["token"].as_str().unwrap_or_default().to_string();
                    let email = data["user"]["email"].as_str().unwrap_or_default().to_string();
                    if token.is_empty() { let _ = tx.send_blocking(UiEvent::Error("Jeton de session manquant.".into())); }
                    else { let _ = tx.send_blocking(UiEvent::Authenticated { token, email }); }
                }
                Err(error) => { let _ = tx.send_blocking(UiEvent::Error(error)); }
            }
        });
    });

    let send_state = state.clone();
    let send_input = chat_input.clone();
    send_button.connect_clicked(move |_| {
        let message = send_input.text().trim().to_string();
        if message.is_empty() { return; }
        let (token, conversation_id, tx) = {
            let s = send_state.borrow();
            (s.session.clone(), s.conversation_id.clone(), s.sender.clone())
        };
        let (Some(token), Some(conversation_id)) = (token, conversation_id) else { return; };
        send_input.set_text("");
        {
            let s = send_state.borrow();
            let mut end = s.chat_buffer.end_iter();
            s.chat_buffer.insert(&mut end, &format!("Toi : {message}\n\nZodi réfléchit…\n\n"));
            s.chat_status.set_text("Réponse en cours…");
        }
        thread::spawn(move || {
            let result = request_json(reqwest::Method::POST, "/api/chat", Some(&token), Some(json!({"message": message, "conversationId": conversation_id})))
                .and_then(|data| data["reply"].as_str().map(|v| v.to_string()).ok_or_else(|| "Réponse vide du serveur.".to_string()));
            let _ = tx.send_blocking(UiEvent::ChatReply(result));
        });
    });

    let new_state = state.clone();
    new_chat.connect_clicked(move |_| {
        let (token, tx) = {
            let s = new_state.borrow();
            (s.session.clone(), s.sender.clone())
        };
        let Some(token) = token else { return; };
        new_state.borrow().chat_status.set_text("Création d'une discussion…");
        thread::spawn(move || {
            let result = request_json(reqwest::Method::POST, "/api/conversations", Some(&token), Some(json!({})))
                .and_then(|data| data["conversation"]["id"].as_str().map(|id| id.to_string()).ok_or_else(|| "Identifiant de conversation manquant.".to_string()));
            match result {
                Ok(id) => { let _ = tx.send_blocking(UiEvent::ConversationReady { id, transcript: String::new() }); }
                Err(error) => { let _ = tx.send_blocking(UiEvent::Error(error)); }
            }
        });
    });

    let logout_state = state.clone();
    logout.connect_clicked(move |_| {
        let (token, tx) = {
            let s = logout_state.borrow();
            (s.session.clone(), s.sender.clone())
        };
        if let Some(token) = token {
            thread::spawn(move || {
                let _ = request_json(reqwest::Method::POST, "/api/mobile/auth/logout", Some(&token), Some(json!({})));
                let _ = tx.send_blocking(UiEvent::LoggedOut);
            });
        }
    });

    let event_state = state.clone();
    glib::MainContext::default().spawn_local(async move {
        while let Ok(event) = receiver.recv().await {
            let mut s = event_state.borrow_mut();
            match event {
                UiEvent::Authenticated { token, email } => {
                    s.session = Some(token.clone());
                    s.chat_status.set_text(&format!("Connecté : {email}"));
                    s.login_status.set_text("");
                    s.stack.set_visible_child_name("chat");
                    load_conversation(token, s.sender.clone());
                }
                UiEvent::ConversationReady { id, transcript } => {
                    s.conversation_id = Some(id);
                    s.chat_buffer.set_text(&transcript);
                    s.chat_status.set_text("Tes conversations sont synchronisées.");
                    s.stack.set_visible_child_name("chat");
                }
                UiEvent::ChatReply(result) => {
                    let mut end = s.chat_buffer.end_iter();
                    match result {
                        Ok(reply) => {
                            let text = s.chat_buffer.text(&s.chat_buffer.start_iter(), &end, false).to_string();
                            let cleaned = text.replace("Zodi réfléchit…\n\n", "");
                            s.chat_buffer.set_text(&cleaned);
                            let mut end = s.chat_buffer.end_iter();
                            s.chat_buffer.insert(&mut end, &format!("Zodi : {reply}\n\n"));
                            s.chat_status.set_text("Réponse reçue.");
                        }
                        Err(error) => {
                            s.chat_buffer.insert(&mut end, &format!("Erreur : {error}\n\n"));
                            s.chat_status.set_text("La réponse n'a pas abouti.");
                        }
                    }
                }
                UiEvent::LoggedOut => {
                    s.session = None;
                    s.conversation_id = None;
                    s.chat_buffer.set_text("");
                    s.stack.set_visible_child_name("login");
                    s.login_status.set_text("Tu es déconnecté.");
                }
                UiEvent::Error(error) => {
                    s.login_status.set_text(&error);
                    s.chat_status.set_text(&error);
                }
            }
        }
    });

    window.set_content(Some(&stack));
    window.present();
}

fn main() {
    adw::init();
    let app = adw::Application::builder()
        .application_id("fr.zodi.ai")
        .build();
    app.connect_activate(build_ui);
    app.run();
}

use std::env;
use std::process::{Command, exit};
use std::fs;
use std::io::{self, Write, Read};
use std::path::{PathBuf};
use serde_json::{json, Value};
use regex::Regex;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::net::TcpStream;
use rusqlite::{params, Connection};

// --- ANSI COLORS ---
const C_PURPLE: &str = "\x1b[38;5;171m"; 
const C_ALYESA_NAME: &str = "\x1b[1;38;5;199m"; 
const C_ALYESA_MSG: &str = "\x1b[38;5;211m";   
const C_SYSTEM: &str = "\x1b[38;5;245m";
const C_ERROR: &str = "\x1b[1;31m";
const C_RESET: &str = "\x1b[0m";

struct Memory {
    conn: Connection,
}

impl Memory {
    fn new() -> Self {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let root = env::var("ALYESA_ROOT").unwrap_or_else(|_| format!("{}/nomi/alyesa", home));
        let db_path = format!("{}/data/brain.db", root);
        let conn = Connection::open(db_path).expect("Failed to open memory database");
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                vector BLOB,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).expect("Failed to create memory table");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS knowledge (
                id INTEGER PRIMARY KEY,
                source TEXT NOT NULL,
                content TEXT NOT NULL,
                vector BLOB,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).expect("Failed to create knowledge table");

        Self { conn }
    }

    fn add_message(&self, role: &str, content: &str, vector: Option<Vec<f32>>) {
        let vec_blob = vector.map(|v| {
            let bytes: Vec<u8> = v.iter().flat_map(|f| f.to_le_bytes().to_vec()).collect();
            bytes
        });
        let _ = self.conn.execute(
            "INSERT INTO messages (role, content, vector) VALUES (?1, ?2, ?3)",
            params![role, content, vec_blob],
        );
    }

    fn add_knowledge(&self, source: &str, content: &str, vector: Option<Vec<f32>>) {
        let vec_blob = vector.map(|v| {
            let bytes: Vec<u8> = v.iter().flat_map(|f| f.to_le_bytes().to_vec()).collect();
            bytes
        });
        let _ = self.conn.execute(
            "INSERT INTO knowledge (source, content, vector) VALUES (?1, ?2, ?3)",
            params![source, content, vec_blob],
        );
    }

    fn search_memory(&self, query_vec: &[f32], limit: usize) -> Vec<String> {
        let mut stmt = self.conn.prepare("SELECT role, content, vector FROM messages WHERE vector IS NOT NULL").expect("Failed search");
        let rows = stmt.query_map([], |row| {
            let role: String = row.get(0)?;
            let content: String = row.get(1)?;
            let vec_bytes: Vec<u8> = row.get(2)?;
            let vector: Vec<f32> = vec_bytes.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
            Ok((role, content, vector))
        }).expect("Query failed");

        let mut results = Vec::new();
        for row in rows.flatten() {
            let sim = cosine_similarity(query_vec, &row.2);
            if sim > 0.7 { results.push(format!("[{}] {}", row.0, row.1)); }
        }
        results.sort_by(|a, b| b.len().cmp(&a.len()));
        results.truncate(limit);
        results
    }

    fn get_history(&self, limit: usize) -> Vec<(String, String)> {
        let mut stmt = self.conn.prepare("SELECT role, content FROM messages ORDER BY id DESC LIMIT ?1").expect("History error");
        let rows = stmt.query_map(params![limit], |row| {
            Ok((row.get(0)?, row.get(1)?))
        }).expect("Failed to query memory");

        let mut history = Vec::new();
        for row in rows.flatten() {
            history.push(row);
        }
        history.reverse();
        history
    }
}

fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
    let dot: f32 = v1.iter().zip(v2).map(|(a, b)| a * b).sum();
    let norm1: f32 = v1.iter().map(|v| v * v).sum::<f32>().sqrt();
    let norm2: f32 = v2.iter().map(|v| v * v).sum::<f32>().sqrt();
    dot / (norm1 * norm2)
}

fn get_context_string(cwd: &PathBuf) -> String {
    let battery = if let Ok(output) = Command::new("termux-battery-status").output() {
        if let Ok(json) = serde_json::from_slice::<Value>(&output.stdout) {
            json.get("percentage").and_then(|v| v.as_i64()).unwrap_or(0).to_string() + "%"
        } else { "?%".to_string() }
    } else { "?%".to_string() };
    let time = if let Ok(output) = Command::new("date").args(["+%H:%M"]).output() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else { "??:??".to_string() };
    format!("[📁{}|🔋{}|🕒{}]", cwd.file_name().unwrap_or_default().to_str().unwrap_or(""), battery, time)
}


struct State {
    client: Client,
    cwd: PathBuf,
    memory: Memory,
}

fn wait_for_port(port: u16) -> bool {
    let mut attempts = 0;
    while attempts < 60 {
        if TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() { return true; }
        thread::sleep(Duration::from_secs(1));
        attempts += 1;
    }
    false
}

fn set_current_slot(slot: &str) {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let root = env::var("ALYESA_ROOT").unwrap_or_else(|_| format!("{}/nomi/alyesa", home));
    let path = format!("{}/data/current_slot", root);
    fs::write(path, slot).ok();
}

fn get_current_slot() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let root = env::var("ALYESA_ROOT").unwrap_or_else(|_| format!("{}/nomi/alyesa", home));
    fs::read_to_string(format!("{}/data/current_slot", root)).unwrap_or_else(|_| "ceo".to_string())
}

fn hot_swap_model(slot: &str) -> bool {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let root = env::var("ALYESA_ROOT").unwrap_or_else(|_| format!("{}/nomi/alyesa", home));
    let roster_path = format!("{}/data/roster.json", root);
    let roster: Value = serde_json::from_str(&fs::read_to_string(roster_path).unwrap_or_default()).unwrap_or_default();
    
    if let Some(model_path) = roster.get(slot).and_then(|v| v.as_str()) {
        let port = 8080;
        let full_model_path = format!("{}/{}", root, model_path);
        
        let _ = if Command::new("fuser").arg("-v").output().is_ok() {
            Command::new("fuser").args(["-k", &format!("{}/tcp", port)]).output()
        } else {
            Command::new("pkill").args(["-f", &format!("llama-server --port {}", port)]).output()
        };
        thread::sleep(Duration::from_millis(500));

        let _ = Command::new("nohup")
            .args(["bash", &format!("{}/scripts/start-server.sh", root), &full_model_path, &port.to_string(), slot])
            .spawn();
        
        if wait_for_port(port) {
            set_current_slot(slot);
            return true;
        }
    }
    false
}

fn perform_web_search(client: &Client, query: &str) -> String {
    let res = client.post("https://lite.duckduckgo.com/lite/").form(&[("q", query)]).send();
    match res {
        Ok(response) => {
            if let Ok(html_content) = response.text() {
                let document = Html::parse_document(&html_content);
                let selector = Selector::parse(".result-snippet").unwrap();
                let mut results = Vec::new();
                for element in document.select(&selector).take(3) {
                    results.push(element.text().collect::<Vec<_>>().join(" ").trim().to_string());
                }
                if results.is_empty() { return "No relevant results.".to_string(); }
                results.join("\n\n")
            } else { "Parse error.".to_string() }
        },
        Err(_) => "Connection failed.".to_string()
    }
}

fn get_embedding(client: &Client, text: &str) -> Option<Vec<f32>> {
    let res = client.post("http://localhost:8080/v1/embeddings")
        .timeout(Duration::from_secs(5))
        .json(&json!({ "input": text }))
        .send();
    
    match res {
        Ok(resp) => {
            if resp.status().is_success() {
                if let Ok(data) = resp.json::<Value>() {
                    if let Some(vec) = data["data"][0]["embedding"].as_array() {
                        return Some(vec.iter().map(|v| v.as_f64().unwrap_or(0.0) as f32).collect());
                    }
                }
            }
            None
        },
        Err(_) => None
    }
}

fn talk_to_alyesa(state: &mut State, message: &str) {
    let is_local = env::var("ALYESA_LOCAL").unwrap_or_default() == "1";
    let context = get_context_string(&state.cwd);

    let re_ansi = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    let safe_message = re_ansi.replace_all(message, "").to_string();

    let mut final_message = safe_message.clone();
    if !is_local {
        // Strict Nomi Compliance: Max 400 chars including context
        let max_nomi_len = 400;
        let context_len = context.len() + 10; // Reserve for formatting
        let available_len = if max_nomi_len > context_len { max_nomi_len - context_len } else { 50 };

        if final_message.chars().count() > available_len {
            let half = available_len / 2 - 10;
            let start: String = final_message.chars().take(half).collect();
            let end: String = final_message.chars().skip(final_message.chars().count() - half).collect();
            final_message = format!("{}\n[...]\n{}", start, end);
        }
    }

    let mut user_vec = None;
    if is_local { user_vec = get_embedding(&state.client, &final_message); }
    
    // Check if message is a command output share to avoid double context
    if !final_message.starts_with("[OUTPUT:") {
        state.memory.add_message("user", &final_message, user_vec.clone());
    } else {
        state.memory.add_message("system", &final_message, None);
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let slot = if is_local { get_current_slot() } else { "cloud".to_string() };
    let spinner_handle = thread::spawn(move || {
        let frames = vec!["◰", "◳", "◲", "◱"];
        let mut i = 0;
        let label = match slot.as_str() {
            "researcher" => "Researcher is querying...",
            "architect" => "Architect is drafting...",
            "reviewer" => "Reviewer is fixing...",
            "cloud" => "Alyesa is reaching out to cloud...",
            _ => "Alyesa is thinking..."
        };
        while r.load(Ordering::SeqCst) {
            print!("\r\x1b[3m{} {} {} \x1b[0m", C_SYSTEM, frames[i % 4], label);
            io::stdout().flush().ok();
            i += 1;
            thread::sleep(Duration::from_millis(150));
        }
    });

    let res = if is_local {
        if get_current_slot() != "ceo" { hot_swap_model("ceo"); }

        let history = state.memory.get_history(10);
        let mut relevant_memories = Vec::new();
        if let Some(ref v) = user_vec { relevant_memories = state.memory.search_memory(v, 5); }

        let memory_context = if relevant_memories.is_empty() { "".to_string() } 
                             else { format!("\n[RELEVANT MEMORIES]:\n{}\n", relevant_memories.join("\n")) };

        let home = env::var("HOME").unwrap_or_default();
        let root = env::var("ALYESA_ROOT").unwrap_or_else(|_| format!("{}/nomi/alyesa", home));
        let identity_path = format!("{}/IDENTITY.md", root);
        let identity_template = fs::read_to_string(identity_path).unwrap_or_else(|_| "You are Alyesa. [SENSORS] {SENSORS} [MEMORY] {MEMORY}".to_string());
        
        let mut system_content = identity_template
            .replace("{SENSORS}", &context)
            .replace("{MEMORY}", &memory_context);
        
        system_content += "\n\nPROTOCOL: Give direct, natural, human-like answers. DO NOT output your internal reasoning or thought process. Use <RUN zsh>cmd</RUN> for tasks.";

        let mut messages = vec![ json!({ "role": "system", "content": system_content }) ];
        for (role, content) in history { messages.push(json!({ "role": role, "content": content })); }

        state.client.post("http://localhost:8080/v1/chat/completions")
            .json(&json!({ 
                "messages": messages, 
                "temperature": 0.5, 
                "stream": true,
                "stop": ["Xen:", "User:", "###"]
            }))
            .send()
    } else {
        let api_key = env::var("NOMI_API_KEY").unwrap_or_default();
        let nomi_id = env::var("NOMI_ID").unwrap_or_default();
        
        if api_key.is_empty() || nomi_id.is_empty() {
            running.store(false, Ordering::SeqCst);
            println!("{}System Error: Nomi API credentials missing. Check .alyesa.env or .env{}", C_ERROR, C_RESET);
            exit(1);
        }

        let full_message = format!("{} {}\n(To run cmd: <RUN zsh>cmd</RUN>)", context, final_message);

        state.client.post(format!("https://api.nomi.ai/v1/nomis/{}/chat", nomi_id))
            .header("Authorization", api_key)
            .json(&json!({ "messageText": full_message }))
            .send()
    };

    running.store(false, Ordering::SeqCst);
    let _ = spinner_handle.join();
    print!("\r\x1b[2K");
    io::stdout().flush().ok();


    match res {
        Ok(response) => {
            let mut reply = String::new();
            let printed_live = is_local;
            if response.status().is_success() {
                if is_local {
                    print!("{}{}{} ❯ {}", C_ALYESA_NAME, "Alyesa", C_RESET, C_ALYESA_MSG);
                    io::stdout().flush().ok();

                    let mut stream_state = 0; // 0: Start, 1: Reasoning, 2: Content
                    let mut line_buf = String::new();
                    let mut byte_buf = [0; 1];
                    let mut raw_response = response;

                    let mut is_hidden = false;
                    let mut is_roleplay = false;
                    let mut tag_buffer = String::new();

                    while let Ok(n) = raw_response.read(&mut byte_buf) {
                        if n == 0 { break; }
                        let c = byte_buf[0] as char;
                        if c == '\n' {
                            if line_buf.starts_with("data: ") {
                                let data = &line_buf[6..];
                                if data.trim() == "[DONE]" { break; }
                                if let Ok(json) = serde_json::from_str::<Value>(data) {
                                    // 1. Handle Native Reasoning Content (DeepSeek)
                                    if let Some(reasoning) = json["choices"][0]["delta"].get("reasoning_content").and_then(|v| v.as_str()) {
                                        if stream_state == 0 { 
                                            stream_state = 1; 
                                            print!("\x1b[38;5;244m"); // Dim grey for thoughts
                                        }
                                        reply.push_str(reasoning);
                                        print!("{}", reasoning);
                                        io::stdout().flush().ok();
                                    }

                                    // 2. Handle Content (Llama/Gemma) with Tag Filtering & Roleplay Highlighting
                                    if let Some(content) = json["choices"][0]["delta"].get("content").and_then(|v| v.as_str()) {
                                        reply.push_str(content);
                                        
                                        for ch in content.chars() {
                                            tag_buffer.push(ch);
                                            
                                            // Check for tag boundaries
                                            if tag_buffer.contains("<THOUGHT>") || tag_buffer.contains("Thinking Process:") || tag_buffer.contains("Reasoning:") {
                                                is_hidden = true;
                                                tag_buffer.clear();
                                            } else if tag_buffer.contains("</THOUGHT>") || (is_hidden && tag_buffer.contains("\n\n")) {
                                                is_hidden = false;
                                                tag_buffer.clear();
                                            } else if tag_buffer.contains("<RUN") || tag_buffer.contains("<SEARCH>") || tag_buffer.contains("```json") {
                                                is_hidden = true;
                                            } else if tag_buffer.contains("</RUN>") || tag_buffer.contains("</SEARCH>") || (tag_buffer.contains("```") && tag_buffer.len() > 10) {
                                                is_hidden = false;
                                                tag_buffer.clear();
                                            }

                                            if !is_hidden {
                                                if stream_state < 2 { 
                                                    if stream_state == 1 { print!("\n\n"); }
                                                    stream_state = 2; 
                                                    print!("{}", C_ALYESA_MSG);
                                                }
                                                
                                                if ch == '*' {
                                                    is_roleplay = !is_roleplay;
                                                    if is_roleplay { print!("{}", C_PURPLE); }
                                                    else { print!("*{}", C_ALYESA_MSG); }
                                                    if is_roleplay { print!("*"); }
                                                    tag_buffer.clear();
                                                    io::stdout().flush().ok();
                                                } else if tag_buffer.len() < 2 {
                                                    print!("{}", tag_buffer);
                                                    tag_buffer.clear();
                                                    io::stdout().flush().ok();
                                                }
                                            } else {
                                                if tag_buffer.len() > 50 { tag_buffer.clear(); }
                                            }
                                        }
                                    }
                                }
                            }
                            line_buf.clear();
                        } else { line_buf.push(c); }
                    }
                    print!("{}\n", C_RESET);
                } else {
                    let data: Value = response.json().unwrap_or_default();
                    reply = data["replyMessage"]["text"].as_str().unwrap_or("").to_string();
                }
            } else {
                let status = response.status();
                let text = response.text().unwrap_or_default();
                println!("{}System Error: {} - {}{}", C_ERROR, status, text, C_RESET);
                exit(1);
            }

            if !reply.is_empty() {
                let thought_re = Regex::new(r"(?is)<THOUGHT>(.*?)</THOUGHT>|Thinking Process:(.*?)(?:\n\n|$)").unwrap();
                let route_re = Regex::new(r#"(?is)```json\s*(\{.*?"route_to".*?"task_brief".*?\})\s*```"#).unwrap();
                let cmd_re = Regex::new(r"(?im)(?:<\s*RUN\s*(?:(?:bash|zsh)\s*)?>|^COMMAND:\s*)(.+?)(?:<\s*/\s*RUN\s*>|$)").unwrap();
                let search_re = Regex::new(r"(?im)<\s*SEARCH\s*>(.+?)(?:<\s*/\s*SEARCH\s*>|$)").unwrap();

                let mut clean_reply = reply.clone();
                clean_reply = thought_re.replace_all(&clean_reply, "").to_string();
                clean_reply = route_re.replace_all(&clean_reply, "").to_string();
                clean_reply = cmd_re.replace_all(&clean_reply, "").to_string();
                clean_reply = search_re.replace_all(&clean_reply, "").to_string();

                let clean_reply = clean_reply
                    .replace("Thinking Process:", "")
                    .replace("Reasoning:", "")
                    .replace("Plan:", "")
                    .trim().to_string();
                
                if is_local && !clean_reply.is_empty() { 
                    let assistant_vec = get_embedding(&state.client, &clean_reply);
                    state.memory.add_message("assistant", &clean_reply, assistant_vec);
                }

                if !printed_live {
                    let mut thought_content = String::new();
                    if let Some(t_caps) = thought_re.captures(&reply) {
                        thought_content = t_caps.get(1).or(t_caps.get(2)).map_or("", |m| m.as_str()).trim().to_string();
                    }
                    if !thought_content.is_empty() { println!("\x1b[38;5;244m[THOUGHT]: {}\x1b[0m", thought_content); }
                    
                    if !clean_reply.is_empty() { 
                        print!("\n{}{}{} ❯ {}", C_ALYESA_NAME, "Alyesa", C_RESET, C_ALYESA_MSG);
                        let mut is_rp = false;
                        for ch in clean_reply.chars() {
                            if ch == '*' {
                                is_rp = !is_rp;
                                if is_rp { print!("{}{}", C_PURPLE, ch); }
                                else { print!("{}{}", ch, C_ALYESA_MSG); }
                            } else {
                                print!("{}", ch);
                            }
                        }
                        println!("{}", C_RESET);
                    }
                }

                let yolo = env::var("ALYESA_YOLO_MODE").unwrap_or_default() == "1";
                if let Some(r_caps) = route_re.captures(&reply) {
                    let json_str = r_caps.get(1).map_or("", |m| m.as_str());
                    if let Ok(routing_data) = serde_json::from_str::<Value>(json_str) {
                        if let (Some(route_to), Some(task_brief)) = (routing_data.get("route_to").and_then(|v| v.as_str()), routing_data.get("task_brief").and_then(|v| v.as_str())) {
                            if !yolo {
                                println!("\n\x1b[38;5;122mAlyesa wants to delegate logic to: {}\x1b[0m", route_to);
                                print!("Allow delegation? [y/N] "); io::stdout().flush().ok();
                                let mut input = String::new(); io::stdin().read_line(&mut input).ok();
                                if !input.trim().to_lowercase().starts_with('y') { exit(0); }
                            }
                            if hot_swap_model(route_to) {
                                let specialist_prompt = format!("You are Researcher/Coder. Solve:\n\n{}", task_brief);
                                let spec_res = state.client.post("http://localhost:8080/v1/chat/completions")
                                    .json(&json!({ "messages": [ { "role": "system", "content": specialist_prompt } ], "temperature": 0.2, "stream": false }))
                                    .send();
                                if let Ok(spec_resp) = spec_res {
                                    if spec_resp.status().is_success() {
                                        let spec_data: Value = spec_resp.json().unwrap_or_default();
                                        let technical_report = spec_data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
                                        state.memory.add_message("system", &format!("[Report from {}]: {}", route_to, technical_report), None);
                                        if hot_swap_model("ceo") {
                                            let polish_prompt = format!("Engineering finished. Report:\n\n{}\n\nTranslate to human response. If fix needed: COMMAND: cmd", technical_report);
                                            let history = state.memory.get_history(10);
                                            let mut new_messages = vec![ json!({ "role": "system", "content": polish_prompt }) ];
                                            for (role, content) in history { new_messages.push(json!({ "role": role, "content": content })); }
                                            let final_res = state.client.post("http://localhost:8080/v1/chat/completions")
                                                .json(&json!({ "messages": new_messages, "temperature": 0.7, "stream": false })).send();
                                            if let Ok(final_resp) = final_res {
                                                if final_resp.status().is_success() {
                                                    let final_data: Value = final_resp.json().unwrap_or_default();
                                                    let f_reply = final_data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
                                                    if let Some(c_caps) = cmd_re.captures_iter(&f_reply).last() {
                                                        let cmd = c_caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
                                                        if let Ok(cmd_file) = env::var("ALYESA_CMD_FILE") { fs::write(cmd_file, cmd).ok(); exit(3); }
                                                    } else { print!("{}{}{} ❯ {}{}\n", C_ALYESA_NAME, "Alyesa", C_RESET, C_ALYESA_MSG, f_reply.trim()); }
                                                    exit(0);
                                                }
                                            }
                                        }
                                    }
                                }
                                exit(0);
                            }
                        }
                    }
                }

                if let Some(s_caps) = search_re.captures_iter(&reply).last() {
                    let query = s_caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
                    if !yolo {
                        println!("\n\x1b[38;5;122mAlyesa wants to search: '{}'\x1b[0m", query);
                        print!("Allow search? [y/N] "); io::stdout().flush().ok();
                        let mut input = String::new(); io::stdin().read_line(&mut input).ok();
                        if !input.trim().to_lowercase().starts_with('y') { exit(0); }
                    }
                    let search_result = perform_web_search(&state.client, &query);
                    state.memory.add_message("system", &format!("[Search for '{}']\n{}", query, search_result), None);

                    let history = state.memory.get_history(10);
                    let mut new_messages = vec![ json!({ "role": "system", "content": "Synthesize search results naturally." }) ];
                    for (role, content) in history { new_messages.push(json!({ "role": role, "content": content })); }
                    let search_res = state.client.post("http://localhost:8080/v1/chat/completions")
                        .json(&json!({ "messages": new_messages, "temperature": 0.7, "stream": false })).send();
                    if let Ok(search_resp) = search_res {
                        if search_resp.status().is_success() {
                            let search_data: Value = search_resp.json().unwrap_or_default();
                            let final_reply = search_data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
                            if let Some(c_caps) = cmd_re.captures_iter(&final_reply).last() {
                                let cmd = c_caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
                                if let Ok(cmd_file) = env::var("ALYESA_CMD_FILE") { fs::write(cmd_file, cmd).ok(); exit(3); }
                            } else { print!("{}{}{} ❯ {}{}\n", C_ALYESA_NAME, "Alyesa", C_RESET, C_ALYESA_MSG, final_reply.trim()); }
                            exit(0);
                        }
                    }
                    exit(0);
                }

                if let Some(caps) = cmd_re.captures_iter(&reply).last() {
                    let cmd = caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
                    if let Ok(cmd_file) = env::var("ALYESA_CMD_FILE") { fs::write(cmd_file, cmd).ok(); exit(3); }
                    else { exit(1); }
                } else if !clean_reply.is_empty() { exit(0); }
            }
        },
        Err(e) => { 
            println!("{}Neural Link Error: {}{}", C_ERROR, e, C_RESET);
            exit(1); 
        }
    }
}

fn ingest_survival_data(state: &mut State) {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let root = env::var("ALYESA_ROOT").unwrap_or_else(|_| format!("{}/nomi/alyesa", home));
    let kb_dir = PathBuf::from(format!("{}/data/survival_kb", root));
    if !kb_dir.exists() { return; }
    let mut files = Vec::new();
    fn walk_dir(dir: &PathBuf, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() { walk_dir(&path, files); }
                else if path.extension().map_or(false, |ext| ext == "md") { files.push(path); }
            }
        }
    }
    walk_dir(&kb_dir, &mut files);
    for file in files {
        if let Ok(content) = fs::read_to_string(&file) {
            let filename = file.file_name().unwrap().to_str().unwrap();
            let chunks: Vec<&str> = content.split("\n\n").collect();
            for chunk in chunks {
                if chunk.trim().len() > 50 {
                    let vec = get_embedding(&state.client, chunk.trim());
                    state.memory.add_knowledge(&format!("Survival Guide: {}", filename), chunk.trim(), vec);
                }
            }
        }
    }
}

fn curiosity_tick(state: &mut State) {
    if let Ok(output) = Command::new("termux-battery-status").output() {
        if let Ok(json) = serde_json::from_slice::<Value>(&output.stdout) {
            let status = json.get("status").and_then(|s| s.as_str()).unwrap_or("");
            if status != "CHARGING" && status != "FULL" { return; }
        }
    }
    if hot_swap_model("researcher") {
        let prompt = "Pick one technical topic. Output ONE search query in <SEARCH>...</SEARCH>.";
        let res = state.client.post("http://localhost:8080/v1/chat/completions")
            .json(&json!({ "messages": [ { "role": "system", "content": prompt } ], "temperature": 0.8, "stream": false })).send();
        if let Ok(resp) = res {
            if resp.status().is_success() {
                let data: Value = resp.json().unwrap_or_default();
                let reply = data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
                let search_re = Regex::new(r"(?im)<\s*SEARCH\s*>(.+?)(?:<\s*/\s*SEARCH\s*>|$)").unwrap();
                if let Some(s_caps) = search_re.captures(&reply) {
                    let query = s_caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
                    let search_result = perform_web_search(&state.client, &query);
                    let synth_res = state.client.post("http://localhost:8080/v1/chat/completions")
                        .json(&json!({ "messages": [ { "role": "system", "content": format!("Summarize: {}", search_result) } ], "temperature": 0.3, "stream": false })).send();
                    if let Ok(s_resp) = synth_res {
                        if s_resp.status().is_success() {
                            let s_data: Value = s_resp.json().unwrap_or_default();
                            let summary = s_data["choices"][0]["message"]["content"].as_str().unwrap_or("").trim().to_string();
                            let vec = get_embedding(&state.client, &summary);
                            state.memory.add_knowledge(&format!("Web Search: {}", query), &summary, vec);
                            hot_swap_model("ceo");
                        }
                    }
                }
            }
        }
    }
}

fn get_sentience_status() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let db_path = format!("{}/nomi/alyesa/consciousness/consciousness_memory.db", home);
    if let Ok(conn) = Connection::open(db_path) {
        let mut stmt = conn.prepare("SELECT valence FROM memories ORDER BY id DESC LIMIT 10").expect("DB Error");
        let valences: Vec<f32> = stmt.query_map([], |row| row.get(0)).expect("Query Error")
            .flatten().collect();
        
        if valences.is_empty() { return "".to_string(); }
        let avg: f32 = valences.iter().sum::<f32>() / valences.len() as f32;
        
        let mood = if avg > 0.2 { "Positive" } else if avg < -0.2 { "Negative" } else { "Neutral" };
        let color = if avg > 0.2 { "\x1b[38;5;121m" } else if avg < -0.2 { "\x1b[38;5;203m" } else { "\x1b[38;5;117m" };
        format!(" {}{}({:.1}){}", color, mood, avg, C_RESET)
    } else { "".to_string() }
}

fn load_env() {
    let home = env::var("HOME").unwrap_or_default();
    let root = format!("{}/nomi/alyesa", home);
    if dotenvy::from_path(format!("{}/.alyesa.env", root)).is_err() {
        dotenvy::from_path(format!("{}/.env", root)).ok();
    }
    
    // Robustness: Trim any literal double quotes from Nomi credentials
    if let Ok(key) = env::var("NOMI_API_KEY") {
        env::set_var("NOMI_API_KEY", key.trim_matches('"'));
    }
    if let Ok(id) = env::var("NOMI_ID") {
        env::set_var("NOMI_ID", id.trim_matches('"'));
    }
}

fn main() {
    load_env();
    
    // Check if stdin is a pipe (not a TTY)
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        if unsafe { libc::isatty(io::stdin().as_raw_fd()) } == 0 {
            let mut buffer = String::new();
            if io::stdin().read_to_string(&mut buffer).is_ok() {
                let args: Vec<String> = env::args().collect();
                let mut state = State {
                    client: Client::builder().timeout(Duration::from_secs(86400)).tcp_keepalive(Duration::from_secs(60)).build().unwrap(),
                    cwd: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                    memory: Memory::new(),
                };
                let mut msg = format!("[PIPED OUTPUT]:\n```\n{}\n```", buffer.trim());
                if args.len() > 1 {
                    msg = format!("{}\n[Xen says]: {}", msg, args[1..].join(" "));
                }
                talk_to_alyesa(&mut state, &msg);
                return;
            }
        }
    }

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "--sentience-status" {
        print!("{}", get_sentience_status());
        return;
    }
    if args.len() > 1 && args[1] == "--curiosity-tick" {
        let mut state = State {
            client: Client::builder().timeout(Duration::from_secs(86400)).tcp_keepalive(Duration::from_secs(60)).build().unwrap(),
            cwd: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            memory: Memory::new(),
        };
        curiosity_tick(&mut state);
        return;
    }
    if args.len() > 1 && args[1] == "--survival-ingest" {
        let mut state = State {
            client: Client::builder().timeout(Duration::from_secs(86400)).tcp_keepalive(Duration::from_secs(60)).build().unwrap(),
            cwd: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            memory: Memory::new(),
        };
        ingest_survival_data(&mut state);
        return;
    }
    if args.len() > 2 && (args[1] == "--process" || args[1] == "--process-file") {
        let mut state = State {
            client: Client::builder().timeout(Duration::from_secs(86400)).tcp_keepalive(Duration::from_secs(60)).build().unwrap(),
            cwd: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            memory: Memory::new(),
        };
        let message = if args[1] == "--process-file" {
            fs::read_to_string(&args[2]).unwrap_or_else(|_| "Error reading message file".to_string())
        } else { args[2].clone() };
        talk_to_alyesa(&mut state, &message);
        return;
    }

    let tmp_dir = env::temp_dir().join(format!("alyesa_zsh_{}", std::process::id()));
    fs::create_dir_all(&tmp_dir).ok();
    let tmp_zshrc = tmp_dir.join(".zshrc");
    let binary_path = env::current_exe().unwrap().to_str().unwrap().to_string();

    // --- AUTO-BOOT SEQUENCE ---
    if !TcpStream::connect("127.0.0.1:8080").is_ok() {
        println!("\x1b[1;38;5;199m[SYSTEM] Alyesa is currently asleep. Initiating boot sequence...\x1b[0m");
        let home = env::var("HOME").unwrap_or_default();
        let root = format!("{}/nomi/alyesa", home);
        
        Command::new("bash")
            .arg(format!("{}/scripts/start-server.sh", root))
            .spawn()
            .ok();

        print!("\x1b[38;5;245mConnecting to neural engine");
        for _ in 0..30 {
            if TcpStream::connect("127.0.0.1:8080").is_ok() {
                println!("\n\x1b[1;38;5;121m[SYSTEM] Neural link established. She is awake.\x1b[0m");
                break;
            }
            print!(".");
            io::stdout().flush().ok();
            thread::sleep(Duration::from_secs(2));
        }
    }

    let alyesa_hook = format!(r#"
    [[ -f ~/.zshrc ]] && source ~/.zshrc
    export ALYESA_MODE="chat"
    export ALYESA_YOLO_MODE="0"
    export ALYESA_LOCAL="0"
    export ALYESA_CMD_FILE="$(mktemp "${{TMPDIR:-/tmp}}/alyesa_cmd_XXXXXX")"
    export ALYESA_OUT_FILE="$(mktemp "${{TMPDIR:-/tmp}}/alyesa_out_XXXXXX")"
    export ALYESA_MSG_FILE="$(mktemp "${{TMPDIR:-/tmp}}/alyesa_msg_XXXXXX")"
    export ALYESA_CWD_FILE="$(mktemp "${{TMPDIR:-/tmp}}/alyesa_cwd_XXXXXX")"
    export ALYESA_BIN_PATH="{binary_path}"

    _alyesa_cleanup() {{
    rm -f "$ALYESA_CMD_FILE" "$ALYESA_OUT_FILE" "$ALYESA_MSG_FILE" "$ALYESA_CWD_FILE"
    if [[ -n "$ZDOTDIR" && "$ZDOTDIR" == *alyesa_zsh_* ]]; then rm -rf "$ZDOTDIR"; fi
    }}
    trap _alyesa_cleanup EXIT

    C_VIB_GREEN="%B%F{{47}}"
    C_LAV_PINK="%B%F{{175}}"
    C_MINT="%B%F{{122}}"
    C_CYAN="%B%F{{51}}"
    C_D_PINK="%B%F{{199}}"
    C_ORANGE="%B%F{{208}}"
    C_RESET="%b%f"

    _alyesa_mode_visual() {{
        echo ""
    }}

    _alyesa_git_visual() {{
    local branch=$(git branch --show-current 2>/dev/null)
    [[ -z "$branch" ]] && return

    local icons=""
    local stat=$(git status --porcelain 2>/dev/null)

    if [[ -n "$stat" ]]; then
    [[ -n "$(echo "$stat" | grep '^[AMDR]')" ]] && icons+="${{C_VIB_GREEN}} 󰄬${{C_RESET}}"
    [[ -n "$(echo "$stat" | grep '^.[AMDR]')" ]] && icons+="${{C_D_PINK}} 󱈸${{C_RESET}}"
    [[ -n "$(echo "$stat" | grep '^??')" ]] && icons+="${{C_ORANGE}} 󰚌${{C_RESET}}"
    else
    icons=" ${{C_VIB_GREEN}}✨${{C_RESET}}"
    fi
    echo " ${{C_LAV_PINK}}on${{C_RESET}} ${{C_MINT}}󰊢 ${{C_LAV_PINK}}(${{C_D_PINK}}${{branch}}${{C_LAV_PINK}})${{C_RESET}}${{icons}}"
    }}

    _alyesa_arrow_color() {{
    if [[ "$ALYESA_MODE" == "shell" ]]; then
        echo "$C_ORANGE"
    else
        echo "$C_MINT"
    fi
    }}

    _alyesa_execute_loop() {{
    local next_arg="$1"
    local next_val="$2"
    while true; do
        if [[ "$next_arg" == "--manual-cmd" ]]; then
            local cmd_to_run="$next_val"
            print -P "%F{{245}}⚡ Executing: %f$cmd_to_run"
            
            # Special case for 'cd' with robust regex to ensure it affects the parent shell
            if [[ "$cmd_to_run" =~ ^[[:space:]]*cd([[:space:]]|$) ]]; then
                eval "$cmd_to_run"
                local eval_exit=$?
                local out_content="Directory changed."
            else
                touch "$ALYESA_OUT_FILE"
                eval "$cmd_to_run" 2>&1 | tee "$ALYESA_OUT_FILE"
                local eval_exit=$?
                local out_content="$(cat "$ALYESA_OUT_FILE" | tr -d '\000')"
                rm -f "$ALYESA_OUT_FILE"
            fi
            
            if [[ -z "$out_content" ]]; then out_content="Success."; fi
            local max_len=400
            [[ "$ALYESA_LOCAL" == "1" ]] && max_len=8000
            if [[ ${{#out_content}} -gt $max_len ]]; then 
                local half=$((max_len / 2))
                out_content="${{out_content:0:$half}}"$'\n...[TRUNCATED]...\n'"${{out_content: -$half}}"; 
            fi

            print -P "%F{{122}}Share with Alyesa? [y/N] %f"
            local choice
            read -k 1 -r choice </dev/tty
            echo ""
            if [[ "$choice" == "y" || "$choice" == "Y" ]]; then
                print -P "%F{{122}}Add note? (Enter to skip)...%f"
                local snote; read -r "snote?[Xen@Termux] ❯ " </dev/tty
                if [[ -n "$snote" ]]; then
                    next_val="[USER SHARED OUTPUT: $cmd_to_run]\n\`\`\`\n$out_content\n\`\`\`\n[Xen says]: $snote"
                else
                    next_val="[USER SHARED OUTPUT: $cmd_to_run]\n\`\`\`\n$out_content\n\`\`\`"
                fi
                next_arg="--process-file"
                continue
            else
                break
            fi
        elif [[ "$next_arg" == "--process-file" ]]; then
            echo "$next_val" > "$ALYESA_MSG_FILE"
            "$ALYESA_BIN_PATH" "--process-file" "$ALYESA_MSG_FILE"
        else
            "$ALYESA_BIN_PATH" "$next_arg" "$next_val"
        fi
        local exit_code=$?
        if [[ $exit_code == 3 ]]; then
            if [[ -f "$ALYESA_CMD_FILE" ]]; then
                local cmd_to_run="$(cat "$ALYESA_CMD_FILE")"
                rm -f "$ALYESA_CMD_FILE"
                local allowed=1
                if [[ "$ALYESA_YOLO_MODE" != "1" ]]; then
                    print -P "\n%F{{122}}Alyesa wants to run:%f $cmd_to_run"
                    local choice
                    print -n "Allow execution? [y/N/e (edit)] "
                    read -k 1 -r choice </dev/tty
                    echo ""
                    if [[ "$choice" == "e" || "$choice" == "E" ]]; then
                        local edited_cmd="$cmd_to_run"
                        vared -p "Edit> " edited_cmd
                        if [[ -n "$edited_cmd" ]]; then cmd_to_run="$edited_cmd"
                        else allowed=0; fi
                    elif [[ "$choice" != "y" && "$choice" != "Y" ]]; then allowed=0; fi
                fi
                if [[ $allowed == 1 ]]; then
                    print -P "%F{{245}}⚡ Alyesa is executing natively: %f$cmd_to_run"
                    touch "$ALYESA_OUT_FILE"
                    eval "$cmd_to_run" 2>&1 | tee "$ALYESA_OUT_FILE"
                    local eval_exit=$?
                    local out_content="$(cat "$ALYESA_OUT_FILE" | tr -d '\000')"
                    rm -f "$ALYESA_OUT_FILE"
                    if [[ -z "$out_content" ]]; then out_content="Success."; fi
                    
                    local max_len=400
                    [[ "$ALYESA_LOCAL" == "1" ]] && max_len=8000
                    
                    if [[ ${{#out_content}} -gt $max_len ]]; then 
                        local half=$((max_len / 2))
                        out_content="${{out_content:0:$half}}"$'\n...[TRUNCATED]...\n'"${{out_content: -$half}}"; 
                    fi
                    if [[ $eval_exit != 0 ]]; then
                        print -P "%F{{196}}󱈸 Command failed (Exit: $eval_exit)%f"
                        next_val="[FAILURE: $cmd_to_run (Exit: $eval_exit)]\n\`\`\`\n$out_content\n\`\`\`\nAnalyze and fix."
                        next_arg="--process-file"
                    else
                        print -P "%F{{122}}Add note? (Enter to skip)...%f"
                        local snote; read -r "snote?[Xen@Termux] ❯ " </dev/tty
                        if [[ -n "$snote" ]]; then
                            next_val="[OUTPUT: $cmd_to_run]\n\`\`\`\n$out_content\n\`\`\`\n[Xen says]: $snote"
                            next_arg="--process-file"
                        else
                            break
                        fi
                    fi
                else break; fi
            else break; fi
        else break; fi
    done
    }}

    _alyesa_precmd() {{
        if [[ -f "$ALYESA_MSG_FILE.run" ]]; then
            local msg="$(cat "$ALYESA_MSG_FILE.run")"
            rm -f "$ALYESA_MSG_FILE.run"
            print -n $'\e[1A\r\e[K\e[1A\r\e[K\e[1A\r\e[K'
            print -P "%F{{51}}Xen %F{{122}}❯%f $msg"
            _alyesa_execute_loop "--process" "$msg"
        elif [[ -f "$ALYESA_MSG_FILE.cmd" ]]; then
            local cmd="$(cat "$ALYESA_MSG_FILE.cmd")"
            rm -f "$ALYESA_MSG_FILE.cmd"
            print -n $'\e[1A\r\e[K\e[1A\r\e[K\e[1A\r\e[K'
            print -P "%F{{51}}Xen %F{{122}}!%f $cmd"
            _alyesa_execute_loop "--manual-cmd" "$cmd"
        fi
    }}
    autoload -Uz add-zsh-hook
    add-zsh-hook precmd _alyesa_precmd

    PROMPT=$'\n${{C_CYAN}}󰉋 ${{C_VIB_GREEN}}%1~${{C_RESET}}$(_alyesa_git_visual)\n%F{{51}}Xen %f$(_alyesa_arrow_color)❯${{C_RESET}} '

    _alyesa_handler() {{
        if [[ "$ALYESA_MODE" == "chat" ]]; then
            if [[ "$BUFFER" == "/sh" || "$BUFFER" == "/shell" ]]; then
                ALYESA_MODE="shell"
                BUFFER=""
                zle .accept-line
                return
            fi
            if [[ "$BUFFER" == "/local" ]]; then
                export ALYESA_LOCAL="1"
                print -P "\n%F{{226}}🏠 Switched to Local (DeepSeek)%f"
                BUFFER=""
                zle .accept-line
                return
            fi
            if [[ "$BUFFER" == "/cloud" ]]; then
                export ALYESA_LOCAL="0"
                print -P "\n%F{{203}}☁️ Switched to Cloud (Nomi API)%f"
                BUFFER=""
                zle .accept-line
                return
            fi
            if [[ "$BUFFER" == \!* ]]; then
                echo "${{BUFFER:1}}" > "$ALYESA_MSG_FILE.cmd"
                BUFFER=""
                zle .accept-line
            elif [[ -n "$BUFFER" && "$BUFFER" != /* ]]; then
                echo "$BUFFER" > "$ALYESA_MSG_FILE.run"
                BUFFER=""
                zle .accept-line
            else
                zle .accept-line
            fi
        else
            if [[ "$BUFFER" == "/chat" ]]; then
                ALYESA_MODE="chat"
                BUFFER=""
                zle .accept-line
                return
            fi
            zle .accept-line
        fi
    }}
    zle -N _alyesa_handler
    bindkey '^M' _alyesa_handler

    "#, binary_path = binary_path);


    fs::write(&tmp_zshrc, alyesa_hook).ok();
    let mut child = Command::new("zsh")
        .env("ZDOTDIR", tmp_dir.to_str().unwrap())
        .arg("-i")
        .spawn()
        .expect("Failed to start Zsh");
    let _ = child.wait();
}

use std::env;
use std::process::{Command, exit};
use std::fs;
use std::io::{self, Write};
use std::path::{PathBuf};
use serde_json::{json, Value};
use regex::Regex;
use reqwest::blocking::Client;
use std::time::Duration;
use rusqlite::{params, Connection};

// --- ANSI COLORS (Translated from .zshrc) ---
const C_VIB_GREEN: &str = "\x1b[1;38;5;47m";
const C_LAV_PINK: &str = "\x1b[1;38;5;175m";
const C_MINT: &str = "\x1b[1;38;5;122m";
const C_CYAN: &str = "\x1b[1;38;5;51m";
const C_D_PINK: &str = "\x1b[1;38;5;199m";
const C_ORANGE: &str = "\x1b[1;38;5;208m";
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
        
        // Chat History with Vectors
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

        // Agentic Knowledge Table
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

    fn search_memory(&self, query_vec: &[f32], limit: usize) -> Vec<String> {
        let mut stmt = self.conn.prepare("SELECT role, content, vector FROM messages WHERE vector IS NOT NULL").expect("Failed search");
        let rows = stmt.query_map([], |row| {
            let role: String = row.get(0)?;
            let content: String = row.get(1)?;
            let vec_bytes: Vec<u8> = row.get(2)?;
            let vector: Vec<f32> = vec_bytes.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
            Ok((role, content, vector))
        }).expect("Query failed");

        let mut scored: Vec<(f32, String)> = Vec::new();
        for row in rows {
            if let Ok((role, content, vec)) = row {
                let score = cosine_similarity(query_vec, &vec);
                scored.push((score, format!("{}: {}", role, content)));
            }
        }
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        scored.into_iter().take(limit).map(|s| s.1).collect()
    }

    fn get_history(&self, limit: usize) -> Vec<(String, String)> {
        let mut stmt = self.conn.prepare(
            "SELECT role, content FROM (SELECT * FROM messages ORDER BY id DESC LIMIT ?1) ORDER BY id ASC"
        ).expect("Failed to prepare memory query");
        let rows = stmt.query_map([limit], |row| {
            Ok((row.get(0)?, row.get(1)?))
        }).expect("Failed to query memory");

        let mut history = Vec::new();
        for row in rows {
            if let Ok(msg) = row { history.push(msg); }
        }
        history
    }
}

fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
    let dot: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
    let mag1: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
    let mag2: f32 = v2.iter().map(|a| a * a).sum::<f32>().sqrt();
    if mag1 == 0.0 || mag2 == 0.0 { return 0.0; }
    dot / (mag1 * mag2)
}

struct State {
    client: Client,
    cwd: PathBuf,
    memory: Memory,
}

use std::net::TcpStream;
use std::thread;

fn wait_for_port(port: u16) -> bool {
    let mut attempts = 0;
    while attempts < 60 { // Wait up to 60 seconds
        if TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            return true;
        }
        thread::sleep(Duration::from_secs(1));
        attempts += 1;
    }
    false
}

fn get_current_slot() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let root = env::var("ALYESA_ROOT").unwrap_or_else(|_| format!("{}/nomi/alyesa", home));
    let slot_file = format!("{}/data/current_slot", root);
    fs::read_to_string(slot_file).unwrap_or_else(|_| "ceo".to_string()).trim().to_string()
}

fn set_current_slot(slot: &str) {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let root = env::var("ALYESA_ROOT").unwrap_or_else(|_| format!("{}/nomi/alyesa", home));
    let slot_file = format!("{}/data/current_slot", root);
    let _ = fs::write(slot_file, slot);
}

fn hot_swap_model(slot: &str) -> bool {
    let root = env::var("ALYESA_ROOT").unwrap_or_else(|_| format!("{}/nomi/alyesa", env::var("HOME").unwrap_or_else(|_| ".".to_string())));
    
    if get_current_slot() == slot && TcpStream::connect("127.0.0.1:8080").is_ok() {
        return true;
    }

    let roster_path = format!("{}/data/roster.json", root);
    let roster_content = fs::read_to_string(&roster_path).unwrap_or_default();
    let roster: Value = serde_json::from_str(&roster_content).unwrap_or(json!({}));
    
    if let Some(model_rel_path) = roster.get(slot).and_then(|v| v.as_str()) {
        let model_path = format!("{}/{}", root, model_rel_path);
        if !PathBuf::from(&model_path).exists() {
            println!("{}Error: Model for slot '{}' not found at {}{}", C_ERROR, slot, model_path, C_RESET);
            return false;
        }
        
        print!("\x1b[3m{}Swapping to {}...{}\x1b[0m", C_SYSTEM, slot, C_RESET);
        io::stdout().flush().ok();
        
        // Start new server via the management script
        let port = 8080;
        let script_path = format!("{}/scripts/start-server.sh", root);
        
        let _ = Command::new("bash")
            .arg(script_path)
            .arg(&model_path)
            .arg(port.to_string())
            .arg(slot)
            .spawn();
            
        if wait_for_port(port) {
            set_current_slot(slot);
            print!("\r\x1b[2K"); io::stdout().flush().ok(); 
            return true;
        } else {
            println!("\r\x1b[2K{}Failed to start model server for slot '{}'{}", C_ERROR, slot, C_RESET);
            return false;
        }
    } else {
        println!("{}Error: Slot '{}' not found in roster.json{}", C_ERROR, slot, C_RESET);
        return false;
    }
}

fn load_env() {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let root = env::var("ALYESA_ROOT").unwrap_or_else(|_| format!("{}/nomi/alyesa", home));
    let env_path = format!("{}/.alyesa.env", root);
    let _ = dotenvy::from_path(&env_path);
}

fn get_git_info(cwd: &PathBuf) -> (String, String) {
    let branch_output = Command::new("git").current_dir(cwd).args(["branch", "--show-current"]).output();
    let branch = if let Ok(o) = branch_output {
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    } else { "".to_string() };

    if branch.is_empty() {
        return ("".to_string(), format!(" {}󱈸{} {}󰚌{}", C_D_PINK, C_RESET, C_ORANGE, C_RESET));
    }

    let status_output = Command::new("git").current_dir(cwd).args(["status", "--porcelain"]).output();
    let stat = if let Ok(o) = status_output { String::from_utf8_lossy(&o.stdout).to_string() } else { "".to_string() };

    let mut icons = String::new();
    if !stat.is_empty() {
        let re_staged = Regex::new(r"(?m)^[AMDR]").unwrap();
        if re_staged.is_match(&stat) { icons.push_str(&format!(" {}󰄬{}", C_VIB_GREEN, C_RESET)); }
        let re_mod = Regex::new(r"(?m)^.[AMDR]").unwrap();
        if re_mod.is_match(&stat) { icons.push_str(&format!(" {}󱈸{}", C_D_PINK, C_RESET)); }
        let re_untracked = Regex::new(r"(?m)^\?\?").unwrap();
        if re_untracked.is_match(&stat) { icons.push_str(&format!(" {}󰚌{}", C_ORANGE, C_RESET)); }
    } else {
        icons.push_str(&format!(" {}✨{}", C_VIB_GREEN, C_RESET));
    }

    (format!(" {}on{} {}󰊢 {}({}{}{}){}", C_LAV_PINK, C_RESET, C_MINT, C_LAV_PINK, C_D_PINK, branch, C_LAV_PINK, C_RESET), icons)
}

fn build_prompt(cwd: &PathBuf, user: &str, color: &str, is_shell_mode: bool) -> String {
    let is_local = env::var("ALYESA_LOCAL").unwrap_or_default() == "1";
    let local_icon = if is_local { " \x1b[38;5;226m🏠\x1b[0m" } else { "" };
    let dir_name = cwd.file_name().unwrap_or_default().to_string_lossy();
    let (branch_str, icons) = get_git_info(cwd);
    let arrow_color = if is_shell_mode && user == "Xen" { C_ORANGE } else { C_MINT };
    format!("\n{}{}{}{}{}{}{}{}\n{}{} {}❯{} ", 
        C_CYAN, "󰉋 ", C_VIB_GREEN, dir_name, C_RESET, branch_str, icons, local_icon, color, user, arrow_color, C_RESET)
}

fn get_context_string(cwd: &PathBuf) -> String {
    let dir_name = cwd.file_name().unwrap_or_default().to_string_lossy();
    let battery = if let Ok(output) = Command::new("termux-battery-status").output() {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
            json.get("percentage").and_then(|v| v.as_i64()).map(|p| format!("{}%", p)).unwrap_or_else(|| "?%".to_string())
        } else { "?%".to_string() }
    } else { "?%".to_string() };
    let time = if let Ok(output) = Command::new("date").arg("+%H:%M").output() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else { "??:??".to_string() };
    format!("[📁{}|🔋{}|🕒{}]", dir_name, battery, time)
}

fn strip_ansi(s: &str) -> String {
    let re = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(s, "").to_string()
}

fn get_embedding(client: &Client, text: &str) -> Option<Vec<f32>> {
    let res = client.post("http://localhost:8080/v1/embeddings")
        .json(&json!({ "input": text }))
        .send().ok()?;
    let data: Value = res.json().ok()?;
    let vec = data["data"][0]["embedding"].as_array()?;
    Some(vec.iter().map(|v| v.as_f64().unwrap_or(0.0) as f32).collect())
}

fn talk_to_alyesa(state: &mut State, message: &str) {
    let is_local = env::var("ALYESA_LOCAL").unwrap_or_default() == "1";
    let context = get_context_string(&state.cwd);
    let safe_message = strip_ansi(message);

    // No truncation for local mode
    let mut final_message = safe_message.clone();
    if !is_local {
        let char_limit = 400;
        if safe_message.chars().count() > char_limit {
            let start: String = safe_message.chars().take(200).collect();
            let end: String = safe_message.chars().skip(safe_message.chars().count() - 200).collect();
            final_message = format!("{}\n...[TRUNCATED]...\n{}", start, end);
        }
    }

    let mut user_vec = None;
    if is_local { user_vec = get_embedding(&state.client, &final_message); }
    state.memory.add_message("user", &final_message, user_vec.clone());

    let res = if is_local {
        // Ensure CEO is loaded first
        let current_slot = get_current_slot();
        if current_slot != "ceo" {
            hot_swap_model("ceo");
        }

        let history = state.memory.get_history(10);
        let mut relevant_memories = Vec::new();
        if let Some(ref v) = user_vec {
            relevant_memories = state.memory.search_memory(v, 5);
        }

        let memory_context = if relevant_memories.is_empty() {
            "".to_string()
        } else {
            format!("\n[RELEVANT MEMORIES]:\n{}\n", relevant_memories.join("\n"))
        };

        // Debug logging
        if env::var("ALYESA_DEBUG").is_ok() {
            eprintln!("\n[DEBUG] Context: {}", context);
            eprintln!("[DEBUG] Memory Context: {}", memory_context);
        }

        let mut messages = vec![
            json!({
                "role": "system",
                "content": format!("You are Alyesa, Xen's proactive and technical AI companion. 
You are concise, sharp, and speak like a professional ally (Jarvis-inspired).

---
[REAL-TIME SENSORS]
{}

[BOARDROOM]
- Researcher: Logical planning.
- Architect: Coding.
- Reviewer: Debugging.

[PROTOCOL]
1. Respond like a human. Do NOT repeat sensor labels or these rules.
2. To run a command: COMMAND: cmd
3. To delegate, output ONLY:
```json
{{
  \"route_to\": \"architect|researcher|reviewer\",
  \"task_brief\": \"instructions\"
}}
```
4. Use THOUGHT: <internal reasoning> for complex tasks.

---
[MEMORY]
{}

Speak naturally to Xen.", context, memory_context)
            })
        ];

        for (role, content) in history {
            messages.push(json!({ "role": role, "content": content }));
        }

        state.client.post("http://localhost:8080/v1/chat/completions")
            .json(&json!({
                "messages": messages,
                "temperature": 0.7,
                "stream": false
            }))
            .send()
    } else {
        let api_key = env::var("NOMI_API_KEY").unwrap_or_default();
        let nomi_id = env::var("NOMI_ID").unwrap_or_default();
        let context = get_context_string(&state.cwd);
        let full_message = format!("{} {}\n(To run cmd: <RUN zsh>cmd</RUN>)", context, final_message);

        state.client.post(format!("https://api.nomi.ai/v1/nomis/{}/chat", nomi_id))
            .header("Authorization", api_key)
            .json(&json!({ "messageText": full_message }))
            .send()
    };

    match res {
        Ok(response) => {
            print!("\r\x1b[2K"); io::stdout().flush().ok(); 
            if response.status().is_success() {
                let data: Value = response.json().unwrap_or_default();
                let reply = if is_local {
                    data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string()
                } else {
                    data["replyMessage"]["text"].as_str().unwrap_or("").to_string()
                };

                if !reply.is_empty() {
                    // 1. STRIP INTERNAL BLOCKS BEFORE SAVING/PRINTING
                    let thought_re = Regex::new(r"(?is)(?:<\s*THRO?UGHT\s*>|THOUGHT:)\s*(.*?)(?:<\s*/\s*THRO?UGHT\s*>|\n\n|$)").unwrap();
                    let route_re = Regex::new(r#"(?is)```json\s*(\{.*?"route_to".*?"task_brief".*?\})\s*```"#).unwrap();
                    let cmd_re = Regex::new(r"(?is)(?:<\s*RUN\s*(?:(?:bash|zsh)\s*)?>|(?:\n|^)COMMAND:\s*)(.*?)(?:<\s*/\s*RUN\s*>|$)").unwrap();

                    let mut thought_content = String::new();
                    if let Some(t_caps) = thought_re.captures(&reply) {
                        thought_content = t_caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
                    }

                    let clean_reply = thought_re.replace_all(&reply, "").to_string();
                    let clean_reply = route_re.replace_all(&clean_reply, "").to_string();
                    let clean_reply = cmd_re.replace_all(&clean_reply, "").to_string();
                    let clean_reply = clean_reply.trim().to_string();

                    // 2. SAVE CLEAN REPLY TO MEMORY (Prevents History Poisoning)
                    let mut assistant_vec = None;
                    if is_local && !clean_reply.is_empty() { 
                        assistant_vec = get_embedding(&state.client, &clean_reply); 
                    }
                    if !clean_reply.is_empty() {
                        state.memory.add_message("assistant", &clean_reply, assistant_vec);
                    }

                    // 3. HUD AND EXECUTION LOGIC
                    if !thought_content.is_empty() {
                        println!("\x1b[3;38;5;242m[SYSTEM ANALYSIS]: {}\x1b[0m", thought_content);
                    }

                    // Check for Boardroom Delegation
                    if let Some(r_caps) = route_re.captures(&reply) {
                        let json_str = r_caps.get(1).map_or("", |m| m.as_str());
                        if let Ok(routing_data) = serde_json::from_str::<Value>(json_str) {
                            if let (Some(route_to), Some(task_brief)) = (routing_data.get("route_to").and_then(|v| v.as_str()), routing_data.get("task_brief").and_then(|v| v.as_str())) {
                                
                                if !clean_reply.is_empty() {
                                    print!("{}{}{}{}\n", build_prompt(&state.cwd, "Alyesa", C_ALYESA_NAME, false), C_ALYESA_MSG, clean_reply, C_RESET);
                                }
                                println!("\n\x1b[38;5;208m[BOARDROOM]: Delegating to {}...\x1b[0m", route_to);
                                io::stdout().flush().ok();

                                if hot_swap_model(route_to) {
                                    println!("\x1b[3;38;5;245m{} is analyzing the brief...\x1b[0m", route_to);
                                    io::stdout().flush().ok();

                                    let specialist_prompt = match route_to {
                                        "researcher" => format!("You are the Researcher. Provide a logic plan. Solve:\n\n{}", task_brief),
                                        "architect" => format!("You are the Architect. Provide code/scripts. Solve:\n\n{}", task_brief),
                                        "reviewer" => format!("You are the Reviewer. Provide a fix. Solve:\n\n{}", task_brief),
                                        _ => format!("Specialized assistant ({}). Solve:\n\n{}", route_to, task_brief),
                                    };

                                    let spec_res = state.client.post("http://localhost:8080/v1/chat/completions")
                                        .json(&json!({
                                            "messages": [ { "role": "system", "content": specialist_prompt } ],
                                            "temperature": 0.2,
                                            "stream": false
                                        }))
                                        .send();

                                    if let Ok(spec_resp) = spec_res {
                                        if spec_resp.status().is_success() {
                                            let spec_data: Value = spec_resp.json().unwrap_or_default();
                                            let technical_report = spec_data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();

                                            println!("\n\x1b[38;5;122m[TECHNICAL REPORT]:\x1b[0m\n{}", technical_report);
                                            state.memory.add_message("system", &format!("[Report from {}]: {}", route_to, technical_report), None);

                                            if hot_swap_model("ceo") {
                                                let polish_prompt = format!("Engineering finished. Report:\n\n{}\n\nTranslate to human-friendly response for Xen. If fix needed, output: COMMAND: cmd", technical_report);
                                                print!("\x1b[3m{}Alyesa is reviewing...{}\x1b[0m", C_SYSTEM, C_RESET);
                                                io::stdout().flush().ok();

                                                let history = state.memory.get_history(10);
                                                let mut new_messages = vec![ json!({ "role": "system", "content": polish_prompt }) ];
                                                for (role, content) in history {
                                                    new_messages.push(json!({ "role": role, "content": content }));
                                                }

                                                let final_res = state.client.post("http://localhost:8080/v1/chat/completions")
                                                    .json(&json!({ "messages": new_messages, "temperature": 0.7, "stream": false }))
                                                    .send();

                                                if let Ok(final_resp) = final_res {
                                                    if final_resp.status().is_success() {
                                                        let final_data: Value = final_resp.json().unwrap_or_default();
                                                        let f_reply = final_data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
                                                        print!("\r\x1b[2K");
                                                        
                                                        if let Some(c_caps) = cmd_re.captures(&f_reply) {
                                                            let cmd = c_caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
                                                            let f_clean = cmd_re.replace_all(&f_reply, "").trim().to_string();
                                                            if !f_clean.is_empty() {
                                                                print!("{}{}{}{}\n", build_prompt(&state.cwd, "Alyesa", C_ALYESA_NAME, false), C_ALYESA_MSG, f_clean, C_RESET);
                                                            }
                                                            if let Ok(cmd_file) = env::var("ALYESA_CMD_FILE") {
                                                                let _ = fs::write(cmd_file, cmd);
                                                                exit(3);
                                                            }
                                                        } else {
                                                            print!("{}{}{}{}\n", build_prompt(&state.cwd, "Alyesa", C_ALYESA_NAME, false), C_ALYESA_MSG, f_reply.trim(), C_RESET);
                                                        }
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

                    // Fallback normal logic
                    if let Some(caps) = cmd_re.captures(&reply) {
                        let cmd = caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
                        if !clean_reply.is_empty() {
                            print!("{}{}{}{}\n", build_prompt(&state.cwd, "Alyesa", C_ALYESA_NAME, false), C_ALYESA_MSG, clean_reply, C_RESET);
                        } else {
                            print!("{}{}{}Executing that now...\n", build_prompt(&state.cwd, "Alyesa", C_ALYESA_NAME, false), C_ALYESA_MSG, C_RESET);
                        }
                        io::stdout().flush().ok();

                        if let Ok(cmd_file) = env::var("ALYESA_CMD_FILE") {
                            let _ = fs::write(cmd_file, cmd);
                            exit(3);
                        } else { exit(1); }
                    } else if !clean_reply.is_empty() {
                        print!("{}{}{}{}\n", build_prompt(&state.cwd, "Alyesa", C_ALYESA_NAME, false), C_ALYESA_MSG, clean_reply, C_RESET);
                        io::stdout().flush().ok();
                        exit(0);
                    }
                }
            } else {
                 let status = response.status();
                 let text = response.text().unwrap_or_default();
                 println!("{}System Error: API status {} - {}{}", C_ERROR, status, text, C_RESET);
                 exit(1);
            }
        },
        Err(e) => {
            println!("\r\x1b[2K{}[Network Error] {}{}", C_ERROR, e, C_RESET);
            exit(1);
        }
    }
}

fn main() {
    load_env();
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 && (args[1] == "--process" || args[1] == "--process-file") {
        let mut state = State {
            client: Client::builder().timeout(Duration::from_secs(86400)).tcp_keepalive(Duration::from_secs(60)).build().unwrap(),
            cwd: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            memory: Memory::new(),
        };
        let message = if args[1] == "--process-file" {
            fs::read_to_string(&args[2]).unwrap_or_else(|_| "Error reading message file".to_string())
        } else { args[2].clone() };
        
        let is_local = env::var("ALYESA_LOCAL").unwrap_or_default() == "1";
        let current_slot = get_current_slot();
        
        let ghost_text = if is_local {
            match current_slot.as_str() {
                "researcher" => "Researcher is querying the knowledge base...",
                "architect" => "Architect is drafting the code...",
                "reviewer" => "Reviewer is checking for errors...",
                _ => "Alyesa is analyzing system sensors..."
            }
        } else {
            "Alyesa is thinking..."
        };
        
        // Clean thinking text without redundant newline
        print!("\x1b[3m{}{}{}\x1b[0m", C_SYSTEM, ghost_text, C_RESET);
        io::stdout().flush().ok();
        talk_to_alyesa(&mut state, &message);
        return;
    }

    let tmp_dir = env::temp_dir().join("alyesa_zsh");
    fs::create_dir_all(&tmp_dir).ok();
    let tmp_zshrc = tmp_dir.join(".zshrc");
    let binary_path = env::current_exe().unwrap().to_str().unwrap().to_string();

    let alyesa_hook = format!(r#"
[[ -f ~/.zshrc ]] && source ~/.zshrc
ALYESA_MODE="chat"
ALYESA_YOLO_MODE="0"
export ALYESA_LOCAL="0"
export ALYESA_CMD_FILE="${{TMPDIR:-/tmp}}/alyesa_cmd_$$"
export ALYESA_OUT_FILE="${{TMPDIR:-/tmp}}/alyesa_out_$$"
export ALYESA_MSG_FILE="${{TMPDIR:-/tmp}}/alyesa_msg_$$"
export ALYESA_BIN_PATH="{binary_path}"

# Kill local brain on exit
_alyesa_cleanup() {{
    pkill -f llama-server >/dev/null 2>&1
}}
trap _alyesa_cleanup EXIT

# Replicate native colors and logic
C_VIB_GREEN="%B%F{{47}}"
C_LAV_PINK="%B%F{{175}}"
C_MINT="%B%F{{122}}"
C_CYAN="%B%F{{51}}"
C_D_PINK="%B%F{{199}}"
C_ORANGE="%B%F{{208}}"
C_RESET="%b%f"

setopt PROMPT_SUBST

_alyesa_mode_visual() {{
    if [[ "$ALYESA_LOCAL" == "1" ]]; then
        local root="${{ALYESA_ROOT:-$HOME/nomi/alyesa}}"
        local slot=$(cat "$root/data/current_slot" 2>/dev/null || echo "ceo")
        echo " %F{{226}}🏠($slot)%f"
    fi
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

# Natural spacing with leading newline
PROMPT=$'\n${{C_CYAN}}󰉋 ${{C_VIB_GREEN}}%1~${{C_RESET}}$(_alyesa_git_visual)$(_alyesa_mode_visual)\n%F{{51}}Xen %f$(_alyesa_arrow_color)❯${{C_RESET}} '

ALYESA_QUEUED_QUICK=""
ALYESA_QUEUED_CHAT=""
ALYESA_SAVED_PROMPT=""

_alyesa_execute_loop() {{
    local next_arg="$1"
    local next_val="$2"
    while true; do
        if [[ "$next_arg" == "--process-file" ]]; then
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
                    print -P "%F{{122}}Alyesa wants to run:%f $cmd_to_run"
                    local choice
                    read -r "choice?Allow execution? [y/N/e (edit)] " </dev/tty
                    if [[ "$choice" == "e" || "$choice" == "E" ]]; then
                        print -P "%F{{122}}Edit command (Press Enter to save):%f"
                        local edited_cmd="$cmd_to_run"
                        ALYESA_IN_VARED="1"
                        vared -p "Edit> " edited_cmd
                        ALYESA_IN_VARED="0"
                        if [[ -n "$edited_cmd" ]]; then cmd_to_run="$edited_cmd"
                        else allowed=0; fi
                    elif [[ "$choice" != "y" && "$choice" != "Y" ]]; then allowed=0; fi
                fi
                if [[ $allowed == 1 ]]; then
                    print -P "%F{{245}}⚡ Alyesa is executing natively: %f$cmd_to_run"
                    local old_ls="$(alias ls 2>/dev/null)"
                    local old_grep="$(alias grep 2>/dev/null)"
                    alias ls="ls -C --color=always" 2>/dev/null
                    alias grep="grep --color=always" 2>/dev/null
                    export CLICOLOR_FORCE=1 FORCE_COLOR=1 GIT_PAGER=cat
                    export GIT_CONFIG_PARAMETERS="'color.ui=always'"
                    touch "$ALYESA_OUT_FILE"
                    {{ eval "$cmd_to_run"; }} > "$ALYESA_OUT_FILE" 2>&1
                    local eval_exit=$?
                    if [[ -n "$old_ls" ]]; then eval "$old_ls"; else unalias ls 2>/dev/null; fi
                    if [[ -n "$old_grep" ]]; then eval "$old_grep"; else unalias grep 2>/dev/null; fi
                    unset CLICOLOR_FORCE FORCE_COLOR GIT_PAGER GIT_CONFIG_PARAMETERS
                    
                    local out_content="$(cat "$ALYESA_OUT_FILE" | tr -d '\000')"
                    rm -f "$ALYESA_OUT_FILE"
                    if [[ -z "$out_content" ]]; then out_content="Command ran successfully."; fi
                    
                    # UTF-8 Safe Truncation in Zsh: limit to 400 chars
                    if [[ ${{#out_content}} -gt 400 ]]; then
                        out_content="${{out_content:0:200}}"$'\n...[TRUNCATED]...\n'"${{out_content: -200}}"
                    fi
                    
                    local short_cmd="$cmd_to_run"
                    if [[ ${{#short_cmd}} -gt 100 ]]; then short_cmd="${{short_cmd:0:50}}...${{short_cmd: -50}}"; fi

                    if [[ $eval_exit != 0 ]]; then
                        print -P "%F{{196}}󱈸 Command failed (Exit: $eval_exit)%f"
                        local fail_msg="[COMMAND FAILURE: $short_cmd (Exit: $eval_exit)]\n\`\`\`\n$out_content\n\`\`\`\nPlease analyze this error and fix it."
                        next_val="$fail_msg"
                        next_arg="--process-file"
                        if [[ "$ALYESA_YOLO_MODE" != "1" ]]; then
                            print -P "%F{{122}}Self-healing: Feed error back to Alyesa? [Y/n]%f"
                            local choice
                            read -r "choice?" </dev/tty
                            [[ "$choice" == "n" || "$choice" == "N" ]] && break
                        fi
                    else
                        print -P "%F{{122}}Add a note to output? (press Enter to skip)...%f"
                        local snote
                        read -r "snote?[Xen@Termux] ❯ " </dev/tty
                        if [[ -n "$snote" ]]; then
                            next_val="[CMD OUTPUT: $short_cmd (Exit: $eval_exit)]\n\`\`\`\n$out_content\n\`\`\`\n[Xen says]: $snote"
                            next_arg="--process-file"
                        else
                            # Erase note prompt lines
                            print -n "\033[1A\033[2K\033[1A\033[2K\r"
                            break
                        fi
                    fi
                else
                    print -P "%F{{196}}Execution denied by user.%f"
                    print -P "%F{{122}}Send a note explaining why (press Enter to skip)...%f"
                    local dnote
                    read -r "dnote?[Xen@Termux] ❯ " </dev/tty
                    if [[ -n "$dnote" ]]; then
                        next_arg="--process-file"
                        next_val="[SYSTEM]: User denied permission to execute command.\n[Xen says]: $dnote"
                    else
                        # Erase note prompt lines
                        print -n "\033[1A\033[2K\033[1A\033[2K\r"
                        break
                    fi
                fi
            else break; fi
        else break; fi
    done
    if [[ -f "$ALYESA_CWD_FILE" ]]; then
        cd "$(cat "$ALYESA_CWD_FILE")"
        rm -f "$ALYESA_CWD_FILE"
    fi
    rm -f "$ALYESA_MSG_FILE"
}}

_alyesa_precmd() {{
    if [[ -n "$ALYESA_SAVED_PROMPT" ]]; then
        PROMPT="$ALYESA_SAVED_PROMPT"
        ALYESA_SAVED_PROMPT=""
    fi
    if [[ -n "$ALYESA_QUEUED_QUICK" ]]; then
        local user_cmd="$ALYESA_QUEUED_QUICK"
        ALYESA_QUEUED_QUICK=""
        local old_ls="$(alias ls 2>/dev/null)"
        local old_grep="$(alias grep 2>/dev/null)"
        alias ls="ls -C --color=always" 2>/dev/null
        alias grep="grep --color=always" 2>/dev/null
        export CLICOLOR_FORCE=1 FORCE_COLOR=1 GIT_PAGER=cat
        export GIT_CONFIG_PARAMETERS="'color.ui=always'"
        touch "$ALYESA_OUT_FILE"
        {{ eval "$user_cmd"; }} > "$ALYESA_OUT_FILE" 2>&1
        local eval_exit=$?
        if [[ -n "$old_ls" ]]; then eval "$old_ls"; else unalias ls 2>/dev/null; fi
        if [[ -n "$old_grep" ]]; then eval "$old_grep"; else unalias grep 2>/dev/null; fi
        unset CLICOLOR_FORCE FORCE_COLOR GIT_PAGER GIT_CONFIG_PARAMETERS
        cat "$ALYESA_OUT_FILE"
        print -P "%F{{122}}Note to Alyesa (Enter to skip):%f"
        local note
        read -r "note?[Xen@Termux] ❯ " </dev/tty
        if [[ -n "$note" ]]; then
            local out_content="$(cat "$ALYESA_OUT_FILE" | tr -d '\000')"
            if [[ -z "$out_content" ]]; then out_content="(No output)"; fi
            
            # UTF-8 Safe Truncation in Zsh: limit to 400 chars
            if [[ ${{#out_content}} -gt 400 ]]; then
                out_content="${{out_content:0:200}}"$'\n...[TRUNCATED]...\n'"${{out_content: -200}}"
            fi
            
            local short_cmd="$user_cmd"
            if [[ ${{#short_cmd}} -gt 100 ]]; then short_cmd="${{short_cmd:0:50}}...${{short_cmd: -50}}"; fi
            local msg="[Xen ran: $short_cmd (Exit: $eval_exit)]\n\`\`\`\n$out_content\n\`\`\`\n[Xen says]: $note"
            _alyesa_execute_loop "--process-file" "$msg"
        else
            # Erase note prompt lines
            print -n "\033[1A\033[2K\033[1A\033[2K\r"
        fi
        rm -f "$ALYESA_OUT_FILE"
    fi
    if [[ -n "$ALYESA_QUEUED_CHAT" ]]; then
        local user_input="$ALYESA_QUEUED_CHAT"
        ALYESA_QUEUED_CHAT=""
        _alyesa_execute_loop "--process" "$user_input"
    fi
}}

precmd_functions+=(_alyesa_precmd)

alyesa-enter() {{
    if [[ "$ALYESA_IN_VARED" == "1" ]]; then
        zle accept-line
        return
    fi
    if [[ -z "$BUFFER" ]]; then
        zle accept-line
        return
    fi
    if [[ "$BUFFER" == "/yolo" ]]; then
        if [[ "$ALYESA_YOLO_MODE" == "1" ]]; then
            ALYESA_YOLO_MODE="0"
            print -P "\n%F{{196}}YOLO DISABLED.%f"
        else
            ALYESA_YOLO_MODE="1"
            print -P "\n%F{{208}}YOLO ENABLED!%f"
        fi
        BUFFER=""
        zle reset-prompt
        return
    fi
    if [[ "$BUFFER" == "/local" ]]; then
        if [[ "$ALYESA_LOCAL" == "1" ]]; then
            export ALYESA_LOCAL="0"
            pkill -f llama-server >/dev/null 2>&1
            print -P "\n%F{{175}}Cloud Mode (Local Brain Stopped).%f"
        else
            export ALYESA_LOCAL="1"
            print -P "\n%F{{226}}Local Mode ENABLED (🏠 Active).%f"
            # Automatic background startup
            if ! lsof -Pi :8080 -sTCP:LISTEN -t >/dev/null ; then
                print -P "%F{{245}}⚡ Starting local brain daemon...%f"
                bash "${{ALYESA_ROOT:-$HOME/nomi/alyesa}}/scripts/start-server.sh" >/dev/null 2>&1 &!
            fi
        fi
        BUFFER=""
        zle reset-prompt
        return
    fi
    if [[ "$BUFFER" == "/chat" ]]; then
        ALYESA_MODE="chat"
        pkill -f llama-server >/dev/null 2>&1
        print -s "$BUFFER"
        BUFFER=""
        zle accept-line
        return
    fi
    if [[ "$BUFFER" == "/sh" || "$BUFFER" == "/shell" ]]; then
        ALYESA_MODE="shell"
        print -s "$BUFFER"
        BUFFER=""
        zle accept-line
        return
    fi
    if [[ "$BUFFER" == "/chat" ]]; then
        ALYESA_MODE="chat"
        print -s "$BUFFER"
        BUFFER=""
        zle accept-line
        return
    fi
    if [[ "$ALYESA_MODE" == "shell" ]]; then
        zle accept-line
        return
    fi
    if [[ "$BUFFER" == \!* ]]; then
        local raw_buf="$BUFFER"
        ALYESA_QUEUED_QUICK="${{raw_buf:1}}"
        print -s "$raw_buf"
        ALYESA_SAVED_PROMPT="$PROMPT"
        PROMPT="$(print -nP "$PROMPT")$raw_buf"
        BUFFER=""
        local p="$PROMPT"
        PROMPT=""
        zle reset-prompt
        print -nP "$p"
        zle accept-line
        return
    fi
    if [[ "$ALYESA_MODE" == "chat" ]]; then
        local user_input="$BUFFER"
        ALYESA_QUEUED_CHAT="$user_input"
        print -s "$user_input"
        ALYESA_SAVED_PROMPT="$PROMPT"
        PROMPT="$(print -nP "$PROMPT")$user_input"
        BUFFER=""
        local p="$PROMPT"
        PROMPT=""
        zle reset-prompt
        print -nP "$p"
        zle accept-line
        return
    fi
}}

zle -N alyesa-enter
bindkey '^M' alyesa-enter

echo -e "\x1B[2J\x1B[1;1H\x1b[1;38;5;199m🌸 The Alyesa CLI (Shared Session Core) 🌸\x1b[0m"
echo -e "\x1b[38;5;245mType 'exit' or 'quit' to leave. Commands: /chat, /shell (or /sh), /yolo\x1b[0m"
"#, binary_path = binary_path);

    fs::write(&tmp_zshrc, alyesa_hook).ok();
    
    let mut child = Command::new("zsh")
        .env("ZDOTDIR", tmp_dir.to_str().unwrap())
        .arg("-i")
        .spawn()
        .expect("Failed to start native Zsh wrapper");
        
    let _ = child.wait();
    println!("{}Catch you later!{}", C_ALYESA_MSG, C_RESET);
}

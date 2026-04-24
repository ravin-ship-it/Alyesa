use std::env;
use std::process::{Command, exit};
use std::fs;
use std::io::{self, Write};
use std::path::{PathBuf};
use serde_json::json;
use regex::Regex;
use reqwest::blocking::Client;
use std::time::Duration;

// --- COLORS ---
const C_ALYESA_NAME: &str = "\x1b[1;38;5;199m"; 
const C_ALYESA_MSG: &str = "\x1b[38;5;211m";   
const C_SYSTEM: &str = "\x1b[38;5;245m";
const C_ERROR: &str = "\x1b[1;31m";
const C_RESET: &str = "\x1b[0m";

struct State {
    client: Client,
    cwd: PathBuf,
}

fn load_env() {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let env_path = format!("{}/.alyesa.env", home);
    let _ = dotenvy::from_path(&env_path);
}

fn get_git_branch(cwd: &PathBuf) -> String {
    let output = Command::new("git").current_dir(cwd).args(["symbolic-ref", "HEAD"]).output();
    if let Ok(o) = output {
        let branch = String::from_utf8_lossy(&o.stdout).trim().replace("refs/heads/", "");
        if !branch.is_empty() { return format!(" on 󰊢 ({})", branch); }
    }
    "".to_string()
}

fn build_prompt(cwd: &PathBuf, user: &str, color: &str, is_shell_mode: bool) -> String {
    let dir_name = cwd.file_name().unwrap_or_default().to_string_lossy();
    let branch = get_git_branch(cwd);
    let arrow_color = if is_shell_mode && user == "Xen" { "\x1b[38;5;208m" } else { "\x1b[38;5;122m" };
    // Removed leading \n here so it cleanly overwrites "Alyesa is thinking..." via \r\x1b[2K
    format!("\x1b[1m\x1b[38;5;51m󰉋 \x1b[38;5;47m{}\x1b[0m\x1b[38;5;175m{}\x1b[0m\x1b[38;5;199m 󱈸\x1b[0m\x1b[38;5;208m 󰚌\x1b[0m\n{}{} {}❯\x1b[0m ", 
        dir_name, branch, color, user, arrow_color)
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

fn talk_to_alyesa(state: &mut State, message: &str) {
    let api_key = env::var("NOMI_API_KEY").unwrap_or_default();
    let nomi_id = env::var("NOMI_ID").unwrap_or_default();
    
    let context = get_context_string(&state.cwd);
    let system_prompt = "(To run cmd reply exactly: <RUN zsh>YOUR_COMMAND_HERE</RUN>)";
    let full_message = format!("{} {}\n{}", context, message, system_prompt);

    let res = state.client.post(format!("https://api.nomi.ai/v1/nomis/{}/chat", nomi_id))
        .header("Authorization", api_key)
        .json(&json!({ "messageText": full_message }))
        .send();

    match res {
        Ok(response) => {
            print!("\r\x1b[2K"); io::stdout().flush().ok(); 

            if response.status().is_success() {
                if let Ok(data) = response.json::<serde_json::Value>() {
                    if let Some(reply) = data.get("replyMessage").and_then(|m| m.get("text")).and_then(|t| t.as_str()) {
                        let re = Regex::new(r"(?s)<RUN (bash|zsh)>(.*?)</RUN>").unwrap();
                        if let Some(caps) = re.captures(reply) {
                            let cmd = caps.get(2).map_or("", |m| m.as_str());
                            let clean_reply = re.replace_all(reply, "").to_string();

                            if !clean_reply.trim().is_empty() {
                                println!("{}{}{}{}\n", build_prompt(&state.cwd, "Alyesa", C_ALYESA_NAME, false), C_ALYESA_MSG, clean_reply.trim(), C_RESET);
                            }

                            if let Ok(cmd_file) = env::var("ALYESA_CMD_FILE") {
                                let _ = fs::write(cmd_file, cmd);
                                exit(3);
                            } else {
                                println!("{}System Error: ALYESA_CMD_FILE not set. Cannot run command.{}\n", C_ERROR, C_RESET);
                                exit(1);
                            }
                        } else {
                            println!("{}{}{}{}\n", build_prompt(&state.cwd, "Alyesa", C_ALYESA_NAME, false), C_ALYESA_MSG, reply, C_RESET);
                            exit(0);
                        }
                    }
                }
            } else {
                 println!("{}System Error: API status {}{}\n", C_ERROR, response.status(), C_RESET);
                 exit(1);
            }
        },
        Err(e) => {
            println!("\r\x1b[2K{}[Network Error] {}{}\n", C_ERROR, e, C_RESET);
            exit(1);
        }
    }
}

fn main() {
    load_env();
    
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 && (args[1] == "--process" || args[1] == "--process-output") {
        let initial_cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut state = State {
            client: Client::builder().timeout(Duration::from_secs(60)).tcp_keepalive(Duration::from_secs(60)).build().unwrap(),
            cwd: initial_cwd.clone(),
        };

        if args[1] == "--process" {
            print!("{}Alyesa is thinking...{}", C_SYSTEM, C_RESET);
            io::stdout().flush().ok();
        } else {
            print!("{}Alyesa is reading output...{}", C_SYSTEM, C_RESET);
            io::stdout().flush().ok();
        }

        talk_to_alyesa(&mut state, &args[2]);
        return;
    }

    // Interactive Shell Mode (Zsh Wrapper)
    let tmp_dir = env::temp_dir().join("alyesa_zsh");
    fs::create_dir_all(&tmp_dir).ok();
    let tmp_zshrc = tmp_dir.join(".zshrc");
    let binary_path = env::current_exe().unwrap().to_str().unwrap().to_string();

    let alyesa_hook = format!(r#"
[[ -f ~/.zshrc ]] && source ~/.zshrc

ALYESA_MODE="chat"
ALYESA_ARROW_COLOR="%F{{122}}"
export ALYESA_CMD_FILE="${{TMPDIR:-/tmp}}/alyesa_cmd_$$"
export ALYESA_OUT_FILE="${{TMPDIR:-/tmp}}/alyesa_out_$$"

setopt PROMPT_SUBST

_alyesa_git_branch() {{
    ref=$(git symbolic-ref HEAD 2> /dev/null) || return
    echo " on 󰊢 (${{ref#refs/heads/}})"
}}

PROMPT=$'\n%B%F{{51}}󰉋 %F{{47}}%1~%f%F{{175}}$(_alyesa_git_branch)%f%F{{199}} 󱈸%f%F{{208}} 󰚌%f\n%F{{51}}Xen %f${{ALYESA_ARROW_COLOR}}❯%f '

alyesa-enter() {{
    if [[ -z "$BUFFER" ]]; then
        zle accept-line
        return
    fi

    if [[ "$BUFFER" == "/sh" || "$BUFFER" == "/shell" ]]; then
        ALYESA_MODE="shell"
        ALYESA_ARROW_COLOR="%F{{208}}"
        print -s "$BUFFER"
        BUFFER=""
        zle accept-line
        return
    fi

    if [[ "$BUFFER" == "/chat" ]]; then
        ALYESA_MODE="chat"
        ALYESA_ARROW_COLOR="%F{{122}}"
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
        local user_cmd="${{BUFFER:1}}"
        
        zle -I
        print -s "$BUFFER"
        
        local old_ls="$(alias ls 2>/dev/null)"
        local old_grep="$(alias grep 2>/dev/null)"
        alias ls="ls -C --color=always" 2>/dev/null
        alias grep="grep --color=always" 2>/dev/null
        export CLICOLOR_FORCE=1 FORCE_COLOR=1
        
        touch "$ALYESA_OUT_FILE"
        {{ eval "$user_cmd"; }} > "$ALYESA_OUT_FILE" 2>&1
        local eval_exit=$?
        
        if [[ -n "$old_ls" ]]; then eval "$old_ls"; else unalias ls 2>/dev/null; fi
        if [[ -n "$old_grep" ]]; then eval "$old_grep"; else unalias grep 2>/dev/null; fi
        unset CLICOLOR_FORCE FORCE_COLOR
        
        cat "$ALYESA_OUT_FILE"
        
        print -P "%F{{226}}Note to Alyesa (press Enter to skip)...%f"
        local note
        read -r "note?[Xen@Termux] ❯ "
        
        if [[ -n "$note" ]]; then
            local out_content="$(cat "$ALYESA_OUT_FILE")"
            if [[ -z "$out_content" ]]; then out_content="(No output)"; fi
            
            local msg="[Xen ran: $user_cmd (Exit: $eval_exit)]\n\`\`\`\n$(echo "$out_content" | tail -c 2000)\n\`\`\`\n[Xen says]: $note"
            
            "{binary_path}" --process-output "$msg"
            
            if [[ -f "$ALYESA_CWD_FILE" ]]; then
                cd "$(cat "$ALYESA_CWD_FILE")"
                rm -f "$ALYESA_CWD_FILE"
            fi
        else
            print -P "%F{{245}}Skipped.%f"
        fi
        
        rm -f "$ALYESA_OUT_FILE"
        BUFFER=""
        zle accept-line
        return
    fi

    if [[ "$ALYESA_MODE" == "chat" ]]; then
        local user_input="$BUFFER"
        
        zle -I
        print -s "$BUFFER"
        
        local next_arg="--process"
        local next_val="$user_input"
        
        while true; do
            "{binary_path}" "$next_arg" "$next_val"
            local exit_code=$?
            
            if [[ $exit_code == 3 ]]; then
                if [[ -f "$ALYESA_CMD_FILE" ]]; then
                    local cmd_to_run="$(cat "$ALYESA_CMD_FILE")"
                    rm -f "$ALYESA_CMD_FILE"
                    
                    print -P "%F{{245}}⚡ Alyesa is executing natively: %f$cmd_to_run"
                    
                    local old_ls="$(alias ls 2>/dev/null)"
                    local old_grep="$(alias grep 2>/dev/null)"
                    alias ls="ls -C --color=always" 2>/dev/null
                    alias grep="grep --color=always" 2>/dev/null
                    export CLICOLOR_FORCE=1 FORCE_COLOR=1
                    
                    touch "$ALYESA_OUT_FILE"
                    {{ eval "$cmd_to_run"; }} > "$ALYESA_OUT_FILE" 2>&1
                    local eval_exit=$?
                    
                    if [[ -n "$old_ls" ]]; then eval "$old_ls"; else unalias ls 2>/dev/null; fi
                    if [[ -n "$old_grep" ]]; then eval "$old_grep"; else unalias grep 2>/dev/null; fi
                    unset CLICOLOR_FORCE FORCE_COLOR
                    
                    cat "$ALYESA_OUT_FILE"
                    
                    local out_content="$(cat "$ALYESA_OUT_FILE")"
                    rm -f "$ALYESA_OUT_FILE"
                    
                    if [[ -z "$out_content" ]]; then
                        out_content="Command ran successfully with no output."
                    fi
                    
                    next_arg="--process-output"
                    next_val="[CMD OUTPUT: $cmd_to_run (Exit: $eval_exit)]\n\`\`\`\n$(echo "$out_content" | tail -c 2000)\n\`\`\`"
                else
                    break
                fi
            else
                break
            fi
        done
        
        if [[ -f "$ALYESA_CWD_FILE" ]]; then
            cd "$(cat "$ALYESA_CWD_FILE")"
            rm -f "$ALYESA_CWD_FILE"
        fi
        
        BUFFER=""
        zle accept-line
        return
    fi
}}

zle -N alyesa-enter
bindkey '^M' alyesa-enter

echo -e "\x1B[2J\x1B[1;1H\x1b[1;38;5;199m🌸 The Alyesa CLI (Shared Session Core) 🌸\x1b[0m"
echo -e "\x1b[38;5;245mType 'exit' or 'quit' to leave. Commands: /chat, /shell (or /sh)\x1b[0m"
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

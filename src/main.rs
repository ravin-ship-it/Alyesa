use std::env;
use std::process::{Command, exit};
use std::fs;
use std::io::{self, Write};
use std::path::{PathBuf};
use serde_json::json;
use regex::Regex;
use reqwest::blocking::Client;
use std::time::Duration;

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

struct State {
    client: Client,
    cwd: PathBuf,
}

fn load_env() {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let env_path = format!("{}/.alyesa.env", home);
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
    let dir_name = cwd.file_name().unwrap_or_default().to_string_lossy();
    let (branch_str, icons) = get_git_info(cwd);
    let arrow_color = if is_shell_mode && user == "Xen" { C_ORANGE } else { C_MINT };
    format!("\n{}{}{} {}{}{}{}\n{}{} {}❯{} ", 
        C_CYAN, "󰉋 ", C_VIB_GREEN, dir_name, C_RESET, branch_str, icons, color, user, arrow_color, C_RESET)
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

fn talk_to_alyesa(state: &mut State, message: &str) {
    let api_key = env::var("NOMI_API_KEY").unwrap_or_default();
    let nomi_id = env::var("NOMI_ID").unwrap_or_default();
    let context = get_context_string(&state.cwd);
    let system_prompt = "(To run cmd: <RUN zsh>cmd</RUN>)";
    
    let safe_message = strip_ansi(message);
    
    // UTF-8 Safe Truncation: Use character counts, not byte counts
    let char_limit = 400;
    let mut final_message = safe_message.clone();
    if safe_message.chars().count() > char_limit {
        let start: String = safe_message.chars().take(200).collect();
        let end: String = safe_message.chars().skip(safe_message.chars().count() - 200).collect();
        final_message = format!("{}\n...[TRUNCATED]...\n{}", start, end);
    }
    
    let full_message = format!("{} {}\n{}", context, final_message, system_prompt);

    let res = state.client.post(format!("https://api.nomi.ai/v1/nomis/{}/chat", nomi_id))
        .header("Authorization", api_key)
        .json(&json!({ "messageText": full_message }))
        .send();

    match res {
        Ok(response) => {
            // Clear thinking message: Move cursor up and wipe line
            print!("\x1b[1A\r\x1b[2K"); io::stdout().flush().ok(); 
            if response.status().is_success() {
                if let Ok(data) = response.json::<serde_json::Value>() {
                    if let Some(reply) = data.get("replyMessage").and_then(|m| m.get("text")).and_then(|t| t.as_str()) {
                        let re = Regex::new(r"(?s)<RUN (bash|zsh)>(.*?)</RUN>").unwrap();
                        if let Some(caps) = re.captures(reply) {
                            let cmd = caps.get(2).map_or("", |m| m.as_str());
                            let clean_reply = re.replace_all(reply, "").to_string();
                            if !clean_reply.trim().is_empty() {
                                println!("{}{}{}{}", build_prompt(&state.cwd, "Alyesa", C_ALYESA_NAME, false), C_ALYESA_MSG, clean_reply.trim(), C_RESET);
                            }
                            if let Ok(cmd_file) = env::var("ALYESA_CMD_FILE") {
                                let _ = fs::write(cmd_file, cmd);
                                exit(3);
                            } else { exit(1); }
                        } else {
                            println!("{}{}{}{}", build_prompt(&state.cwd, "Alyesa", C_ALYESA_NAME, false), C_ALYESA_MSG, reply, C_RESET);
                            exit(0);
                        }
                    }
                }
            } else {
                 let status = response.status();
                 let text = response.text().unwrap_or_default();
                 println!("{}System Error: API status {} - {}{}\n", C_ERROR, status, text, C_RESET);
                 exit(1);
            }
        },
        Err(e) => {
            println!("\x1b[1A\r\x1b[2K{}[Network Error] {}{}\n", C_ERROR, e, C_RESET);
            exit(1);
        }
    }
}

fn main() {
    load_env();
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 && (args[1] == "--process" || args[1] == "--process-file") {
        let mut state = State {
            client: Client::builder().timeout(Duration::from_secs(60)).tcp_keepalive(Duration::from_secs(60)).build().unwrap(),
            cwd: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        };
        let message = if args[1] == "--process-file" {
            fs::read_to_string(&args[2]).unwrap_or_else(|_| "Error reading message file".to_string())
        } else { args[2].clone() };
        // Thinking text in Italic System color
        print!("\n\x1b[3m{}Alyesa is thinking...{}\x1b[0m", C_SYSTEM, C_RESET);
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
export ALYESA_CMD_FILE="${{TMPDIR:-/tmp}}/alyesa_cmd_$$"
export ALYESA_OUT_FILE="${{TMPDIR:-/tmp}}/alyesa_out_$$"
export ALYESA_MSG_FILE="${{TMPDIR:-/tmp}}/alyesa_msg_$$"
export ALYESA_BIN_PATH="{binary_path}"

# Replicate native colors and logic
C_VIB_GREEN="%B%F{{47}}"
C_LAV_PINK="%B%F{{175}}"
C_MINT="%B%F{{122}}"
C_CYAN="%B%F{{51}}"
C_D_PINK="%B%F{{199}}"
C_ORANGE="%B%F{{208}}"
C_RESET="%b%f"

setopt PROMPT_SUBST

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

PROMPT=$'\n${{C_CYAN}}󰉋 ${{C_VIB_GREEN}}%1~${{C_RESET}}$(_alyesa_git_visual)\n%F{{51}}Xen %f$(_alyesa_arrow_color)❯${{C_RESET}} '

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
                    print -P "\n%F{{122}}Alyesa wants to run:%f $cmd_to_run"
                    local choice
                    read -r "choice?Allow execution? [y/N/e (edit)] " </dev/tty
                    print ""
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
                    cat "$ALYESA_OUT_FILE"
                    local out_content="$(cat "$ALYESA_OUT_FILE" | tr -d '\000')"
                    rm -f "$ALYESA_OUT_FILE"
                    if [[ -z "$out_content" ]]; then out_content="Command ran successfully."; fi
                    
                    # UTF-8 Safe Truncation in Zsh: limit to 400 chars
                    if [[ ${{#out_content}} -gt 400 ]]; then
                        out_content="${{out_content:0:200}}"$'\n...[TRUNCATED]...\n'"${{out_content: -200}}"
                    fi
                    
                    print -P "%F{{122}}Add a note to output? (press Enter to skip)...%f"
                    local snote
                    read -r "snote?[Xen@Termux] ❯ " </dev/tty
                    local short_cmd="$cmd_to_run"
                    if [[ ${{#short_cmd}} -gt 100 ]]; then short_cmd="${{short_cmd:0:50}}...${{short_cmd: -50}}"; fi
                    if [[ -n "$snote" ]]; then
                        next_val="[CMD OUTPUT: $short_cmd (Exit: $eval_exit)]\n\`\`\`\n$out_content\n\`\`\`\n[Xen says]: $snote"
                        next_arg="--process-file"
                    else
                        # Erase note prompt lines (Move up 2 and clear)
                        print -n "\033[1A\033[2K\033[1A\033[2K\r"
                        break
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
        print -P "%F{{122}}Note to Alyesa (press Enter to skip)...%f"
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
            print -P "\n%F{{196}}YOLO Mode DISABLED.%f"
        else
            ALYESA_YOLO_MODE="1"
            print -P "\n%F{{208}}YOLO Mode ENABLED!%f"
        fi
        BUFFER=""
        zle reset-prompt
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
        PROMPT="$(print -P "$PROMPT")$raw_buf"
        BUFFER=""
        local p="$PROMPT"
        PROMPT=""
        zle reset-prompt
        print -P "$p"
        zle accept-line
        return
    fi
    if [[ "$ALYESA_MODE" == "chat" ]]; then
        local user_input="$BUFFER"
        ALYESA_QUEUED_CHAT="$user_input"
        print -s "$user_input"
        ALYESA_SAVED_PROMPT="$PROMPT"
        PROMPT="$(print -P "$PROMPT")$user_input"
        BUFFER=""
        local p="$PROMPT"
        PROMPT=""
        zle reset-prompt
        print -P "$p"
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

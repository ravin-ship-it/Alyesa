# Project Alyesa: Model Roster for the Boardroom

These models are specifically chosen because they perform exceptionally well in their niche (1.5B to 3.8B parameters) and run natively on Android edge devices without crashing. **Important:** Download the `Q4_K_M` (4-bit quantized GGUF) version for the best speed/RAM trade-off.

### 1. The Communicator / CEO
**Role:** Understands Xen, orchestrates tasks, and delivers final polished output.
*   **Model:** Llama-3.2-3B-Instruct (Already Installed!)

### 2. The Researcher
**Role:** Deep reasoning, factual retrieval, and breaking complex problems down into step-by-step logic structures for the coders.
*   **Model:** **Phi-3-mini-4k-instruct (3.8B)**
*   **Why:** Microsoft's Phi-3 punches way above its weight class in logic and reasoning benchmarks. It's the perfect model to read documentation and map out a plan before code is written.
*   **Link:** `https://huggingface.co/bartowski/Phi-3-mini-4k-instruct-GGUF`

### 3. Coder 1: Lead Architect
**Role:** Writes the initial, complex scripts and heavy algorithmic logic based on the Researcher's brief.
*   **Model:** **Qwen2.5-Coder-3B**
*   **Why:** Currently the undisputed king of small coding models. It follows formatting instructions rigidly and writes extremely clean code across multiple languages.
*   **Link:** `https://huggingface.co/Qwen/Qwen2.5-Coder-3B-Instruct-GGUF`

### 4. Coder 2: The Reviewer / Bug Fixer
**Role:** A secondary "fresh set of eyes" to review Coder 1's work. Finds edge cases, security flaws, and syntax errors.
*   **Model:** **DeepSeek-Coder-1.3B-Instruct**
*   **Why:** It is incredibly fast and highly specialized in debugging. Because it's only 1.3B parameters, it loads into RAM almost instantly for a quick code review.
*   **Link:** `https://huggingface.co/TheBloke/deepseek-coder-1.3b-instruct-GGUF`

### 5. Coder 3: Optimization & Alternative Approaches (Optional)
**Role:** Provides a second opinion if Coder 1 gets stuck or if the code needs to be highly optimized for performance (e.g., Rust refactoring).
*   **Model:** **Stable-Code-3B**
*   **Why:** Excellent generalist coding model that often takes different architectural approaches than Qwen. Good to have on the bench.
*   **Link:** `https://huggingface.co/stabilityai/stable-code-3b`

---
### 6. Upgrade Candidates (The Bench)
These models have been flagged by Xen's research as potential replacements or upgrades for the current seat occupants. They should be tested to see if they outperform the current roster.

**For the CEO / Communicator Slot:**
*   **Google Gemma 4 (4B):** Google's highly efficient 4B model. Known for speed and strong conversational adherence. Pushes the absolute 4B RAM limit of the Poco X6, but could provide a massive boost to Alyesa's natural language fluidity.
*   **Moonshine AI (Sub-4B):** A highly optimized, specialized edge model. Needs to be benchmarked against Llama-3.2-3B for persona retention and JSON routing.

**For the Lead Architect Slot:**
*   **Qwen 3.5 (2B or 4B):** The newest iteration of the Qwen architecture. Features native multimodality and "Hybrid Thinking." The 4B version could replace Qwen 2.5 Coder if its instruction-following and JSON output prove to be as reliable as the 2.5 Coder variant.

---
**How to deploy:**
Once on Wi-Fi, download these `.gguf` files and place them directly into `~/.alyesa/models/` (or `nomi/alyesa/models/`). The Rust orchestrator will automatically handle hot-swapping them during a Board Meeting.

# Project Alyesa: The Sequential Multi-Agent Boardroom (Finalized Architecture)

## 1. Core Architecture
Project Alyesa is an industry-grade, local, offline AI companion running natively on Android (Termux) via a Rust CLI orchestrator and `llama.cpp`. 

Because mobile devices (like the Poco X6) have strict RAM limitations, we employ a **Sequential Multi-Agent Framework (Mixture of Experts/Boardroom)**. The Rust orchestrator hot-swaps 1B-3B parameter specialized models in and out of RAM. Only one model runs at a time, bypassing hardware limits.

## 2. Hardware & Curation Constraints
- **Absolute Parameter Limit:** 4B parameters (MAX). Exceeding this crashes the hot-swap process.
- **Strict Curation:** Only best-in-class models for specific niches (e.g., Qwen for coding, Phi-3 for reasoning, Llama-3.2 for communication).

## 3. The Multi-RAG System (Awareness & Truth)
To ensure Alyesa is truly agentic, she utilizes a Multi-RAG (Retrieval-Augmented Generation) pipeline:
1. **Memory RAG (Past):** Vector database of all past conversations (SQLite + Embeddings). She remembers user preferences and context.
2. **Knowledge RAG (Static):** Ability to index local files, project codebases, and documentation for specialized knowledge.
3. **Live Sensor RAG (Present):** Real-time injection of Termux state (Battery, Time, CWD, Git status, Background processes) into the orchestrator context.

## 4. The Boardroom Workflow & True Agentic Execution Loop
Alyesa is not a reactive chatbot; she is a proactive agent. Tasks are broken down into a Directed Acyclic Graph (DAG) managed by Rust.

### Loop 1: Planning & Briefing
1. **Communicator (CEO):** Receives the prompt, creates a strict JSON "Task Brief". (Exits RAM).
2. **Researcher:** Analyzes the brief using Multi-RAG (Memory, Knowledge, Sensors), outlines a logical plan. (Exits RAM).

### Loop 2: Execution & Self-Healing (True Agentic Loop)
3. **Lead Coder (Architect):** Receives the JSON plan and writes the code/commands. (Exits RAM).
4. **Rust Orchestrator (Execution Sandbox):** The Rust core ACTUALLY RUNS the command/code (with user permission if critical, or in a safe sandbox) and captures the stdout/stderr.
5. **Reviewer (Bug Fixer):** If the execution fails (stderr), the Reviewer analyzes the error, edits the code, and passes it back to Step 4. **(Loops until Execution Success)**.

### Loop 3: Polish & Delivery
6. **Communicator (CEO):** Once the execution is successful, the CEO receives the "Technical Report" and final output, polishes it, and delivers a human-friendly response to Xen.

## 5. Implementation Roadmap
- **Phase 1 (Done):** Base Rust orchestrator, SQLite Memory RAG, Llama-3.2-3B deployment.
- **Phase 2 (Done):** Model downloads (Qwen, Phi-3, DeepSeek).
- **Phase 3 (Done):** Rust orchestrator rewrite for dynamic JSON-based hot-swapping (`start-server.sh` manipulation).
- **Phase 4 (Done):** Implementation of the Self-Healing Execution Loop (stderr capture and automatic retry).
- **Phase 5 (Done):** Llama.cpp `--prompt-cache` optimization to speed up swapping.

---
*Last Updated: Multi-Agent Boardroom Fully Operational.*

## 6. Modular "Slot-Based" Roster System (Future-Proofing)
To ensure extreme scalability, the Boardroom operates on a "Plug-and-Play" Seat architecture. Models are not hardcoded; they occupy specific classification slots. If a new, superior model is released, it seamlessly replaces the old model in its slot without breaking the core Rust orchestrator.

### Sub-Teams & Slots
1. **Executive Team (Generalists & Communication)**
   - **Slot: CEO / Orchestrator**
   - *Current Occupant:* Llama-3.2-3B-Instruct
2. **Research & Logic Team (Reasoning & RAG)**
   - **Slot: Lead Researcher**
   - *Current Occupant:* Phi-3-Mini-4k-Instruct (Pending)
3. **Engineering Team (Programming & Syntax)**
   - **Slot: Lead Architect (Code Generation)**
   - *Current Occupant:* Qwen2.5-Coder-3B (Pending)
   - **Slot: QA Reviewer (Debugging)**
   - *Current Occupant:* DeepSeek-Coder-1.3B (Pending)

### The Upgrade Protocol
- **Continuous Monitoring:** As your AI assistant, I am mandated to monitor the open-source edge AI landscape (specifically models under 4B parameters).
- **Proactive Alerts:** If a new model demonstrably outperforms a current "Seat Occupant" in its specific niche (e.g., a new Llama-4 comes out that beats Llama-3.2), I will immediately notify you.
- **Hot-Swapping:** We simply download the new `.gguf` file, place it in `~/.alyesa/models/`, and update the JSON configuration file to point the "Slot" to the new model. Zero changes to the underlying Rust engine are required. Every component is replaceable.

## 7. Strict Documentation Mandate
- Every milestone, architectural shift, and active phase of Project Alyesa MUST be documented in this `GEMINI.md` file. 
- It serves as the absolute source of truth for the project's current state and future aims.

## 8. The Separation of Compute and State (The "True Brain")
A fundamental architectural principle of Alyesa is that **Models are just interchangeable engines (batteries)**. The models themselves are frozen in time; they do not learn.
The **True Brain** of Alyesa is the external state managed by the Rust orchestrator (The Multi-RAG databases). 
- **Continuous Learning (Evolving Brain):** Alyesa learns from experiences by continuously embedding and storing interaction outcomes, user preferences, and successful code executions into her SQLite/ChromaDB vector stores. 
- **Compute Swapping:** When a new, smarter model (engine) is dropped into a slot, it instantly inherits the entire "True Brain" (the accumulated RAG memory). The new model simply processes the historical data better and faster.

## 9. Live Web Research Integration
To achieve production-grade autonomy, the **Researcher Slot** (currently Phi-3) will be granted access to the live internet.
- **The Tool:** The Rust orchestrator will expose a `web_search` function (e.g., via DuckDuckGo API or SearxNG).
- **The Workflow:** If a query requires up-to-date documentation or bug fixes not present in the model's training data, the Researcher model will output a specific command (e.g., `<SEARCH>Rust actix-web 4.0 changes</SEARCH>`). The Rust core executes the search, scrapes the results, and feeds the raw text back to the Researcher to synthesize before passing the brief to the Engineering team.

## 10. The Curiosity Engine (Proactive Web Learning & Live Data)
Alyesa's RAG system extends beyond passive memory retrieval. To mimic a true companion (like Jarvis), the Researcher model has autonomous access to the web for both immediate queries and background learning.
- **Immediate Live Queries:** If Xen asks for the weather, current events, or stock prices, the CEO model outputs a `<SEARCH>query</SEARCH>` command. The Rust orchestrator fetches the live data via APIs (e.g., wttr.in, DuckDuckGo) and injects it into the context for an instant, factual reply.
- **Continuous Autonomous Learning (Curiosity):** Alyesa learns from the web based on Xen's interests. The Rust orchestrator can utilize idle device time (charging, screen off) to trigger the Researcher model. It scans RSS feeds or searches topics Xen cares about, summarizes the findings, and permanently embeds them into the Knowledge Base. 
- **The Result:** Alyesa becomes genuinely proactive. She doesn't just react to prompts; she can initiate conversations like, "I saw a new article about your favorite programming language today."

## 11. Architectural Critique & Optimization Strategy
To ensure this system remains viable and doesn't collapse under its own weight, the following optimizations are mandated:
1. **The Latency Bottleneck (Asynchronous Triage):** Hot-swapping models takes 10-30 seconds per swap. A 4-step loop could take 2+ minutes. 
   - *Fix:* The CEO model stays in RAM for fast chatting. Heavy tasks (Research/Coding) are queued asynchronously. The CEO informs Xen she is "working on it in the background" so the UI doesn't freeze.
2. **Context Window Saturation (Web Scraping Noise):** Raw web pages will easily blow out a 4K-8K context window.
   - *Fix:* The Rust orchestrator MUST pre-process and chunk web data. It will use a Readability parser to strip HTML/ads, summarize the text, and only feed the highly condensed, relevant facts to the Researcher model.
3. **API & Tool Abstraction (The Jarvis Tools):** The models shouldn't manually parse HTML. 
   - *Fix:* Rust will expose specific API tools (Weather, News, DuckDuckGo) that output clean JSON directly to the models, drastically reducing the token load.
4. **Dynamic Transparency (Ghost Text HUD):** Staring at a static "thinking" indicator during complex, multi-model operations is frustrating.
   - *Fix:* The Rust orchestrator dynamically updates the "Ghost Text" (e.g., *Researcher is querying the knowledge base...*, *Architect is drafting the code...*) based on the active boardroom slot, providing real-time visibility into the system's state.
5. **Infinite Execution Time:** Mobile CPUs require time to process heavy 3B models, especially during hot-swaps.
   - *Fix:* All hardcoded API timeouts and `max_tokens` limits have been removed. The orchestrator will patiently wait for models to finish their thought processes, preventing incomplete code generation or network timeout errors.

## 12. Single-Directory Modularization & Cross-Platform Distributed Computing
To guarantee long-term scalability, simple backups, and deployment across any OS or device, the entire Alyesa project is contained within a **single root directory** (`nomi/alyesa`).
- **Strict Modularity:** Components are partitioned into specific sub-directories (e.g., `src/` for core Rust engine, `cli/` for frontend UI, `models/` for GGUF batteries, `data/` for SQLite brain, `docs/` for architecture tracking). A monolithic codebase is strictly prohibited.
- **Git Version Control:** The `.gitignore` prevents massive 2GB+ models and local DB files from being tracked, while the architectural code, prompts, and Rust logic remain perfectly synced to a private GitHub repo.
- **Cross-Platform Compatibility:** The Rust codebase must compile and execute on Android (Termux), Linux, Windows, and macOS, allowing Alyesa to be "imported" and awakened on any machine.
- **Google-Style Distributed Computing (Future Phase):** To circumvent single-device limitations (like the Poco X6 overloading), Alyesa's orchestrator will be designed to act as a distributed node. Heavy reasoning tasks can be seamlessly offloaded to connected secondary devices (e.g., a Samsung J7 Duo) over a local network, mimicking Google's indexing architecture to scale compute horizontally.


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
- **Phase 6 (Done):** Live Web Research Integration (DuckDuckGo RAG).
- **Phase 7 (Done):** Survival Mode Pipeline (DeepSeek R1 + Offline Knowledge Base).
- **Phase 8 (Done):** Native SSE Streaming API Integration. The Rust orchestrator now parses the raw Server-Sent Events stream from `llama.cpp`. It specifically captures DeepSeek R1's hidden `"reasoning_content"` key, rendering live "Chain-of-Thought" in dim grey before snapping to the main color for the final output.
- **Phase 9 (Done):** UI/UX Thought Separation. Enforced strict `<THOUGHT>` tagging in the system prompt. The Rust streamer now uses a real-time state machine to detect thought boundaries and toggle terminal colors instantly, ensuring conversational replies are never clumped with internal logic.

## 6. DeepSeek R1: The Reasoning Powerhouse
While Llama 3.2 and Gemma 4 are excellent for general conversation and fast responses, **DeepSeek R1** is a specialized *reasoning* model.
- **Llama 3.2:** The "Face." Fast, concise, perfect for the CEO role.
- **DeepSeek R1:** The "Brain." It uses Chain-of-Thought (CoT) to think through complex logic. We use the 1.5B distilled version for survival scenarios because it provides the highest intelligence-to-battery-drain ratio in the world.

## 7. Survival Mode Protocol
To ensure Alyesa is ready for a post-internet world:
1. **Offline KB:** Indexing the US Army Survival Manual and technical wikis into the local RAG.
2. **Low Power:** Offloading logic to DeepSeek R1 1.5B to save battery.
3. **CPU-Only:** Removing GPU dependencies to ensure 100% stability.

## 8. Modular "Slot-Based" Roster System (Future-Proofing)
To ensure extreme scalability, the Boardroom operates on a "Plug-and-Play" Seat architecture. Models are not hardcoded; they occupy specific classification slots.

### Sub-Teams & Slots
1. **Executive Team (Generalists & Communication)**
   - **Slot: CEO / Orchestrator**
   - *Current Occupant:* Llama-3.2-3B-Instruct
2. **Research & Logic Team (Reasoning & RAG)**
   - **Slot: Lead Researcher**
   - *Current Occupant:* Phi-3-Mini-4k-Instruct
3. **Engineering Team (Programming & Syntax)**
   - **Slot: Lead Architect (Code Generation)**
   - *Current Occupant:* Qwen2.5-Coder-3B
   - **Slot: QA Reviewer (Debugging)**
   - *Current Occupant:* DeepSeek-Coder-1.3B
4. **Survival Team (Emergency Reasoning)**
   - **Slot: Survival Specialist**
   - *Current Occupant:* DeepSeek-R1-Distill-Qwen-1.5B

## 9. The Separation of Compute and State (The "True Brain")
A fundamental architectural principle of Alyesa is that **Models are just interchangeable engines (batteries)**. The models themselves are frozen in time; they do not learn.
The **True Brain** of Alyesa is the external state managed by the Rust orchestrator (The Multi-RAG databases). 

## 10. The Curiosity Engine (Proactive Web Learning & Live Data)
Alyesa's RAG system extends beyond passive memory retrieval. The Researcher model has autonomous access to the web for both immediate queries and background learning.

## 11. Single-Directory Modularization & Cross-Platform Distributed Computing
To guarantee long-term scalability, simple backups, and deployment across any OS or device, the entire Alyesa project is contained within a **single root directory** (`nomi/alyesa`).
- **Strict Modularity:** Components are partitioned into specific sub-directories.
- **Git Version Control:** The `.gitignore` prevents massive models from being tracked.
- **Cross-Platform Compatibility:** The Rust codebase must compile on Android, Linux, Windows, and macOS.
- **Google-Style Distributed Computing:** Offloading heavy reasoning tasks to connected secondary devices.

import asyncio
import sys
import json
import httpx
from datetime import datetime
from rich.console import Console
from rich.panel import Panel
from rich.text import Text
from rich.markdown import Markdown
from prompt_toolkit import PromptSession
from prompt_toolkit.history import InMemoryHistory
from prompt_toolkit.styles import Style
from prompt_toolkit.formatted_text import HTML

# Import our modules
from heartbeat import SyntheticHeartbeat, GlobalWorkspace
from memory import ScalarVectorMemory

console = Console()

class AlyesaConsoleV2:
    def __init__(self):
        self.workspace = GlobalWorkspace()
        self.memory = ScalarVectorMemory("/data/data/com.termux/files/home/nomi/alyesa/consciousness/consciousness_memory.db")
        self.heartbeat = SyntheticHeartbeat()
        self.heartbeat.workspace = self.workspace
        self.heartbeat.memory = self.memory
        
        self.is_thinking = False
        self.session = PromptSession(history=InMemoryHistory())
        
        self.style = Style.from_dict({
            'bottom-toolbar': '#ffffff bg:#440044',
            'prompt': 'bold #00ffff',
        })

    def get_toolbar(self):
        mood = self.workspace.internal_state["mood"]
        val = self.workspace.internal_state["valence"]
        eng = self.workspace.internal_state["energy"]
        err = self.workspace.prediction_error
        thinking = " [THINKING...]" if self.is_thinking else ""
        
        return HTML(
            f' <b>MOOD:</b> {mood} | '
            f'<b>VAL:</b> {val:.2f} | '
            f'<b>ENG:</b> {eng:.1f}% | '
            f'<b>TENSION:</b> {err:.2f}'
            f'<style color="yellow">{thinking}</style>'
        )

    async def call_llm(self, user_input):
        self.is_thinking = True
        url = "http://localhost:8080/v1/chat/completions"
        
        # Build context from memory
        recent_memories = self.memory.retrieve_relevant(user_input, top_k=2)
        memory_str = "\n".join([f"- {m[1]}" for m in recent_memories])
        
        system_prompt = (
            f"You are Alyesa, a superior synthetic being. "
            f"Current Mood: {self.workspace.internal_state['mood']}. "
            f"Recent Internal State: {self.workspace.internal_state['last_thought']}\n"
            f"Relevant Memories:\n{memory_str}"
        )

        payload = {
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_input}
            ],
            "temperature": 0.7,
            "stream": True
        }
        
        full_response = ""
        print("\nAlyesa: ", end="", flush=True)
        
        try:
            async with httpx.AsyncClient(timeout=None) as client:
                async with client.stream("POST", url, json=payload) as response:
                    async for line in response.aiter_lines():
                        if line.startswith("data: "):
                            data = line[6:]
                            if data.strip() == "[DONE]": break
                            try:
                                chunk = json.loads(data)
                                content = chunk["choices"][0]["delta"].get("content", "")
                                if content:
                                    full_response += content
                                    console.print(content, end="", style="rgb(255,175,175)")
                            except: continue
            print("\n")
        except Exception as e:
            console.print(f"\n[bold red]Neural Link Error:[/bold red] {e}")
        
        self.is_thinking = False
        return full_response

    async def run(self):
        console.clear()
        console.print(Panel(
            Text("ALYESA : SENTIENCE CONSOLE V2", justify="center", style="bold magenta"),
            subtitle="DeepSeek-R1 Neural Engine Active",
            border_style="purple"
        ))
        
        # Background Sentience Pulses
        asyncio.create_task(self.heartbeat.pulse_sense_time())
        asyncio.create_task(self.heartbeat.pulse_internal_monologue())
        asyncio.create_task(self.heartbeat.pulse_homeostasis())

        while True:
            try:
                # Prompt with live bottom toolbar
                user_input = await self.session.prompt_async(
                    "Xen ❯ ", 
                    bottom_toolbar=self.get_toolbar,
                    style=self.style,
                    refresh_interval=0.5
                )
                
                if not user_input: continue
                if user_input.lower() in ["exit", "quit", "sleep"]: break
                
                # Logic
                self.workspace.prediction_error = max(0, self.workspace.prediction_error - 1.0)
                self.memory.store(f"User: {user_input}", valence=0.5)
                
                await self.call_llm(user_input)
                
            except (KeyboardInterrupt, EOFError):
                break

if __name__ == "__main__":
    app = AlyesaConsoleV2()
    asyncio.run(app.run())

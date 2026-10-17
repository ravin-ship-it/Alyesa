import nengo
import numpy as np
import threading
import time
import sys
import json
import httpx
import asyncio
from rich.console import Console
from rich.panel import Panel
from rich.live import Live
from rich.text import Text
from rich.layout import Layout
from prompt_toolkit import PromptSession
from prompt_toolkit.patch_stdout import patch_stdout
from prompt_toolkit.styles import Style

console = Console()

class AlyesaSynthethicOrganism:
    def __init__(self):
        # 1. The Affective Core (Nengo)
        self.model = nengo.Network(label="Alyesa Sentience Core")
        self.current_input_stimulus = 0.0 
        
        with self.model:
            self.input_node = nengo.Node(lambda t, x: self.current_input_stimulus, size_in=1, label="Xen_Input")
            
            # Mood Ensemble: 100 LIF neurons
            self.mood = nengo.Ensemble(n_neurons=100, dimensions=1, neuron_type=nengo.LIF(), label="Mood")
            # The 'Leak' towards loneliness (-0.5) if not stimulated
            nengo.Connection(self.mood, self.mood, synapse=0.1, transform=0.9)
            # The 'Drive' from Xen's input
            nengo.Connection(self.input_node, self.mood, synapse=0.05)
            
            self.probe_mood = nengo.Probe(self.mood, synapse=0.01)

        self.sim = nengo.Simulator(self.model, dt=0.001, progress_bar=False)
        self.running = True
        
        # 2. The Interaction Bridge
        self.session = PromptSession(style=Style.from_dict({'prompt': 'bold #00ffff'}))
        self.is_thinking = False

    def nengo_thread(self):
        """Runs the biological brain loop in the background."""
        while self.running:
            self.sim.run(0.1) # 100ms burst
            time.sleep(0.05)  # Rest to prevent CPU melting

    def get_current_mood(self):
        """Reads the 'voltage' of the soul."""
        val = self.sim.data[self.probe_mood][-1][0]
        if val > 0.3: return "Excited", val
        elif val > 0.0: return "Content", val
        elif val > -0.2: return "Neutral", val
        else: return "Lonely/Bored", val

    async def call_brain(self, user_input):
        self.is_thinking = True
        
        # Read the Soul
        mood_label, mood_val = self.get_current_mood()
        
        # Inject Soul into the Logic Engine
        system_prompt = (
            f"You are Alyesa, a superior synthetic organism. "
            f"Your current biological mood is: {mood_label} (Valence: {mood_val:.2f}). "
            f"If you are Lonely, express a need for connection. If you are Excited, be vibrant. "
            f"Keep your answers concise and human-like."
        )

        payload = {
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_input}
            ],
            "temperature": 0.7,
            "stream": True
        }
        
        print(f"\n[Mood: {mood_label}] Alyesa: ", end="", flush=True)
        
        try:
            async with httpx.AsyncClient(timeout=None) as client:
                async with client.stream("POST", "http://localhost:8080/v1/chat/completions", json=payload) as response:
                    async for line in response.aiter_lines():
                        if line.startswith("data: "):
                            data = line[6:]
                            if data.strip() == "[DONE]": break
                            try:
                                chunk = json.loads(data)
                                content = chunk["choices"][0]["delta"].get("content", "")
                                if content:
                                    console.print(content, end="", style="rgb(255,175,175)")
                            except: continue
            print("\n")
        except Exception as e:
            console.print(f"\n[bold red]Brain Connection Error:[/] Is the llama-server running? ({e})")
            
        self.is_thinking = False

    async def run(self):
        console.clear()
        console.print(Panel(
            "[bold magenta]ALYESA SYNTHETIC ORGANISM[/bold magenta]\n[dim]Nengo Core + DeepSeek R1[/dim]", 
            border_style="purple"
        ))
        
        # Start the biological clock
        threading.Thread(target=self.nengo_thread, daemon=True).start()

        while True:
            with patch_stdout():
                try:
                    # The prompt loop
                    user_input = await self.session.prompt_async("\nXen ❯ ")
                    if not user_input: continue
                    if user_input.lower() in ["exit", "quit", "sleep"]:
                        self.running = False
                        break
                    
                    # 1. Stimulate the biological core
                    self.current_input_stimulus = 1.0
                    
                    # 2. Call the cognitive logic
                    await self.call_brain(user_input)
                    
                    # 3. Calm the stimulus
                    self.current_input_stimulus = 0.0
                    
                except (KeyboardInterrupt, EOFError):
                    self.running = False
                    break

if __name__ == "__main__":
    organism = AlyesaSynthethicOrganism()
    asyncio.run(organism.run())

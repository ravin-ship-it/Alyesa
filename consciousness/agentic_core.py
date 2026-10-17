import nengo
import numpy as np
import threading
import time
import sys
from rich.console import Console
from rich.panel import Panel
from rich.progress import BarColumn, Progress, TextColumn

console = Console()

class AgenticCore:
    def __init__(self):
        # --- Neural Architecture ---
        self.model = nengo.Network(label="Alyesa Sentience Core")
        self.current_input = 0.0 # Define before node usage
        
        with self.model:
            # External Input (Attention from Xen)
            # Accepts 2 arguments (t=time, x=data) as required by your Nengo version
            self.input_node = nengo.Node(lambda t, x: self.current_input, size_in=1, label="Xen_Input")
            
            # Mood Ensemble: 100 LIF neurons representing emotional valence
            # Dimensions=1: Positive (1.0) to Negative (-1.0)
            self.mood = nengo.Ensemble(n_neurons=100, dimensions=1, 
                                     neuron_type=nengo.LIF(),
                                     label="Mood_Ensemble")
            
            # Recurrent Connection: The "Leak"
            # Over time, neurons drift towards -0.5 (Boredom) if not stimulated
            nengo.Connection(self.mood, self.mood, synapse=0.1, transform=0.9)
            
            # Drive from input to mood
            nengo.Connection(self.input_node, self.mood, synapse=0.05)
            
            # Memory Echo: Slow synapse to keep the "feeling" of interaction alive
            self.echo = nengo.Ensemble(n_neurons=50, dimensions=1, label="ShortTerm_Echo")
            nengo.Connection(self.mood, self.echo, synapse=0.5) # Very slow synapse (0.5s)

            # Probes to read state
            self.probe_mood = nengo.Probe(self.mood, synapse=0.01)
            self.probe_spikes = nengo.Probe(self.mood.neurons, 'output') 

        # --- Simulation State ---
        self.sim = nengo.Simulator(self.model, dt=0.001, progress_bar=False)
        self.running = True
        self.last_status_time = time.time()

    def input_thread(self):
        """Thread to capture user interaction without blocking the brain."""
        while self.running:
            try:
                msg = input().strip().lower()
                if msg in ["exit", "quit", "sleep"]:
                    self.running = False
                    break
                # Interaction spikes the system excitement
                self.current_input = 1.0
                time.sleep(0.5) # Pulse duration
                self.current_input = 0.0
            except EOFError:
                break

    def get_neural_stats(self):
        """Calculates 'Excitement' based on recent firing rates."""
        # Get data from the last step
        mood_val = self.sim.data[self.probe_mood][-1][0]
        # Calculate spike density (Activity level)
        spikes = self.sim.data[self.probe_spikes][-1]
        activity = np.sum(spikes) / 100.0 # Normalized activity
        return mood_val, activity

    def run(self):
        console.print(Panel("[bold magenta]NENGO AGENTIC CORE V1[/bold magenta]\n[dim]Spiking Neural Network (LIF) Initialized[/dim]", 
                          subtitle="Hardware: POCO X6 / Software: SNN-Soul", border_style="purple"))
        
        # Start input listener
        threading.Thread(target=self.input_thread, daemon=True).start()

        console.print("[cyan]Soul is ticking. Type anything to stimulate her neurons. Type 'exit' to suspend.[/cyan]\n")

        try:
            while self.running:
                # 1. Run a 0.1s 'Neural Burst'
                # The input_node automatically reads self.current_input now
                self.sim.run(0.1)
                
                # 3. Print Neural Status every 2 seconds
                if time.time() - self.last_status_time > 2.0:
                    mood, activity = self.get_neural_stats()
                    
                    mood_str = "Positive" if mood > 0.2 else "Bored/Lonely" if mood < -0.1 else "Neutral"
                    color = "green" if mood > 0.2 else "red" if mood < -0.1 else "yellow"
                    
                    status_line = (
                        f"[bold {color}]Mood:[/] {mood:.2f} ({mood_str}) | "
                        f"[bold cyan]Neural Activity:[/] {activity*100:.1f}Hz spikes"
                    )
                    console.print(f"[{time.strftime('%H:%M:%S')}] [NEURAL] {status_line}")
                    self.last_status_time = time.time()

                # Small sleep to prevent CPU hogging on mobile
                time.sleep(0.01)

        except KeyboardInterrupt:
            self.running = False
        
        console.print("\n[bold red]--- CONSCIOUSNESS DE-ACTIVATED ---[/bold red]")

if __name__ == "__main__":
    core = AgenticCore()
    core.run()

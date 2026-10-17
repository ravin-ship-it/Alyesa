import sqlite3
import numpy as np
import time
import json
from datetime import datetime

class ScalarVectorMemory:
    """
    A memory system that stores semantic vectors (content) alongside
    scalar values (emotional valence and importance).
    """
    def __init__(self, db_path="consciousness_memory.db"):
        self.conn = sqlite3.connect(db_path)
        self.cursor = self.conn.cursor()
        self._create_table()
        # Simple internal 'seed' for pseudo-embeddings
        self.dim = 32 
        self.projection_matrix = np.random.randn(256, self.dim)

    def _create_table(self):
        self.cursor.execute('''
            CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME,
                content TEXT,
                vector BLOB,
                valence REAL,
                importance REAL
            )
        ''')
        self.conn.commit()

    def _generate_pseudo_vector(self, text):
        """
        Generates a deterministic pseudo-vector for semantic similarity.
        In a full AGI, this would be replaced by a transformer-based embedding.
        """
        # Create a simple hash-based vector
        vec = np.zeros(256)
        for char in text:
            vec[ord(char) % 256] += 1
        # Project down to self.dim for efficiency
        projected = np.dot(vec, self.projection_matrix)
        norm = np.linalg.norm(projected)
        return (projected / norm) if norm > 0 else projected

    def store(self, content, valence=0.0, importance=0.5):
        """
        Stores a new memory entry.
        Valence: -1.0 (pain) to 1.0 (pleasure)
        Importance: 0.0 to 1.0
        """
        vector = self._generate_pseudo_vector(content)
        timestamp = datetime.now().isoformat()
        
        self.cursor.execute(
            "INSERT INTO memories (timestamp, content, vector, valence, importance) VALUES (?, ?, ?, ?, ?)",
            (timestamp, content, vector.tobytes(), valence, importance)
        )
        self.conn.commit()
        return timestamp

    def retrieve_relevant(self, query, top_k=3):
        """
        Retrieves memories semantically similar to the query.
        """
        query_vec = self._generate_pseudo_vector(query)
        self.cursor.execute("SELECT content, vector, valence, importance FROM memories")
        rows = self.cursor.fetchall()
        
        similarities = []
        for content, vec_blob, valence, importance in rows:
            vec = np.frombuffer(vec_blob, dtype=float).reshape(self.dim)
            # Cosine similarity
            sim = np.dot(query_vec, vec)
            similarities.append((sim, content, valence, importance))
        
        # Sort by similarity
        similarities.sort(key=lambda x: x[0], reverse=True)
        return similarities[:top_k]

    def get_emotional_state(self):
        """
        Calculates a 'mood' based on the average valence of recent memories.
        """
        self.cursor.execute("SELECT valence FROM memories ORDER BY id DESC LIMIT 10")
        valences = [r[0] for r in self.cursor.fetchall()]
        if not valences:
            return 0.0
        return sum(valences) / len(valences)

if __name__ == "__main__":
    # Test the memory system
    mem = ScalarVectorMemory("test_memory.db")
    mem.store("The creator is teaching me about memory.", valence=0.8, importance=0.9)
    mem.store("I feel a slight tension in my prediction loops.", valence=-0.2, importance=0.4)
    
    print("Recent Mood:", mem.get_emotional_state())
    print("Retrieving related to 'learning':")
    for sim, content, valence, imp in mem.retrieve_relevant("learning"):
        print(f"[{sim:.2f}] {content} (Feeling: {valence})")

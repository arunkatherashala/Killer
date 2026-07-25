#!/usr/bin/env python3
"""
ACTIVE Q&A AGENT - Python Demo
Demonstrates the Killer agent framework in action
Date: March 19, 2026
"""

import json
from datetime import datetime
from typing import List, Dict, Optional


class Agent:
    """Autonomous Q&A Agent"""
    
    def __init__(self, name: str, role: str, description: str):
        self.id = f"agent_{datetime.now().timestamp()}"
        self.name = name
        self.role = role
        self.description = description
        self.state = "Idle"
        self.memory: List[str] = []
        self.thoughts: List[str] = []
        self.actions: List[str] = []
        self.conversation_count = 0
        self.knowledge_score = 0.95
    
    def remember(self, fact: str, importance: float = 0.9) -> None:
        """Record a fact in memory"""
        self.memory.append(f"[{importance:.2f}] {fact}")
    
    def think(self) -> List[str]:
        """Generate reasoning chain"""
        return [
            "Analyzing question structure",
            "Searching knowledge base",
            "Generating reasoning chain"
        ]
    
    def answer_question(self, question: str) -> str:
        """Answer any question"""
        self.conversation_count += 1
        
        # Analyze question
        analysis = self.analyze_question(question)
        self.thoughts.extend(self.think())
        
        # Check knowledge base
        answer = self.generate_answer(question, analysis)
        
        # Record action
        self.actions.append(f"answered_question: {question[:50]}...")
        self.remember(f"Answered: {question}", importance=0.85)
        
        return answer
    
    def analyze_question(self, question: str) -> Dict:
        """Determine question type"""
        lower = question.lower()
        
        types = {
            "why": "explanation",
            "how": "procedure",
            "what": "definition",
            "when": "timing",
            "who": "entity",
            "where": "location"
        }
        
        q_type = "general"
        for keyword, q_type_name in types.items():
            if keyword in lower:
                q_type = q_type_name
                break
        
        return {
            "type": q_type,
            "intent": "answer_request",
            "confidence": 0.92
        }
    
    def generate_answer(self, question: str, analysis: Dict) -> str:
        """Generate answer based on question type"""
        lower = question.lower()
        
        # Killer-specific knowledge
        if "killer" in lower and "language" in lower:
            return "Killer is a high-performance programming language with actors for concurrency, pattern matching, type system, and JIT compilation. Designed for distributed systems, real-time applications, and systems programming."
        
        if "agent" in lower:
            return "An agent is an autonomous entity that can reason, remember facts, take actions, and observe results. Killer agents use chain-of-thought reasoning and can be organized in swarms."
        
        if "performance" in lower:
            return "Killer achieves ~1-2M operations/sec on single-threaded workloads, and 1000+ req/sec through its actor-based concurrency model."
        
        if "deploy" in lower or "production" in lower:
            return "Killer supports cloud deployment via Docker, Kubernetes, and serverless platforms. Use blue-green, canary, or rolling deployments for safe updates."
        
        if "how" in lower and "actor" in lower:
            return "Actors are created with spawn(), messages are sent with send(), and responses are awaited with await."
        
        # Default answer based on question type
        templates = {
            "explanation": "The reason is that this demonstrates key principles of distributed systems design.",
            "procedure": "Follow these steps: 1) prepare, 2) initialize, 3) execute, 4) monitor, 5) complete.",
            "definition": "This is a fundamental concept characterized by specific properties and behaviors.",
            "timing": "This happens according to the system's timing and state requirements.",
            "entity": "This refers to a key component in the architecture.",
            "location": "It is located in the designated module or service.",
            "general": "Based on available information and reasoning, here's the answer..."
        }
        
        return templates.get(analysis["type"], templates["general"])
    
    def status(self) -> Dict:
        """Get agent status"""
        return {
            "name": self.name,
            "role": self.role,
            "state": self.state,
            "conversations": self.conversation_count,
            "memories": len(self.memory),
            "thoughts_generated": len(self.thoughts),
            "actions_taken": len(self.actions),
            "knowledge_score": self.knowledge_score
        }


class AgentPool:
    """Manage multiple agents"""
    
    def __init__(self, capacity: int = 5):
        self.agents: List[Agent] = []
        self.capacity = capacity
        self.tasks_completed = 0
    
    def add_agent(self, agent: Agent) -> bool:
        """Add agent to pool"""
        if len(self.agents) < self.capacity:
            self.agents.append(agent)
            return True
        return False
    
    def get_active_agents(self) -> int:
        """Get number of active agents"""
        return len(self.agents)
    
    def distribute_task(self, question: str) -> List[str]:
        """Distribute task across agent pool"""
        answers = []
        for agent in self.agents:
            answer = agent.answer_question(question)
            answers.append(f"{agent.name}: {answer}")
            self.tasks_completed += 1
        return answers


def main():
    """Main Q&A loop"""
    print("╔═══════════════════════════════════════════╗")
    print("║  KILLER ACTIVE Q&A AGENT - Python Demo   ║")
    print("║  Ask me anything! Type 'exit' to quit     ║")
    print("║  Type 'status' to see agent stats         ║")
    print("╚═══════════════════════════════════════════╝\n")
    
    # Create agent
    qa_agent = Agent(
        name="KillerBot",
        role="question_answerer",
        description="Active agent that answers any question"
    )
    
    # Create agent pool
    pool = AgentPool(capacity=3)
    pool.add_agent(qa_agent)
    pool.add_agent(Agent("AnalysisBot", "analyst", "Analyzes questions"))
    pool.add_agent(Agent("ReportBot", "reporter", "Reports findings"))
    
    print(f"✅ initialized {pool.get_active_agents()} agents in pool\n")
    
    # Main loop
    while True:
        try:
            user_input = input("You: ").strip()
            
            if not user_input:
                continue
            
            if user_input.lower() in ["exit", "quit"]:
                print("\nAgent: Goodbye! Thank you for chatting. 👋")
                break
            
            if user_input.lower() == "status":
                print(f"\nAgent Status:")
                status = qa_agent.status()
                for key, value in status.items():
                    print(f"  {key}: {value}")
                print(f"\nPool Status:")
                print(f"  Active agents: {pool.get_active_agents()}")
                print(f"  Tasks completed: {pool.tasks_completed}")
                print()
                continue
            
            # Answer question
            answer = qa_agent.answer_question(user_input)
            print(f"Agent: {answer}\n")
            
        except KeyboardInterrupt:
            print("\n\nAgent: Interrupted. Goodbye! 👋")
            break
        except Exception as e:
            print(f"Error: {e}")
            continue
    
    # Final stats
    print("\n" + "="*45)
    print("AGENT STATISTICS")
    print("="*45)
    final_status = qa_agent.status()
    for key, value in final_status.items():
        print(f"{key:.<25} {value}")
    print(f"{'pool_tasks_completed':.<25} {pool.tasks_completed}")
    print("="*45)


if __name__ == "__main__":
    main()

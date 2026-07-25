# Killer - Actor Model & Concurrency

## What is the Actor Model?

The actor model is Killer's approach to **safe concurrent programming**. Instead of shared memory with locks, actors communicate through **message passing**.

---

## Basic Actor Example

```killer
actor Greeter {
  handle greet(name: String) -> String {
    "Hello, " + name + "!"
  }
}

kfn main() {
  # Spawn an actor
  greeter = Greeter::spawn()
  
  # Send a message and wait for response
  response = greeter.greet("World").await
  println(response)
}

main()
```

---

## Multiple Actors Communicating

```killer
actor Counter {
  handle increment() -> Int {
    1
  }
  
  handle get_value() -> Int {
    0
  }
}

kfn main() {
  counter = Counter::spawn()
  
  result1 = counter.increment().await
  result2 = counter.increment().await
  value = counter.get_value().await
  
  println("Incremented twice")
}

main()
```

---

## Key Concepts

### Spawning an Actor
```killer
actor MyActor { ... }
instance = MyActor::spawn()
```

### Sending Messages
```killer
result = instance.method_name(args).await
```

### Actor Isolation
- Each actor has **independent state**
- No shared memory = **no locks needed**
- **Type safe** message passing
- **Deterministic** execution

---

## Use Cases

1. **Web servers** - One actor per client connection
2. **Game loops** - Separate actors for game state, rendering, physics
3. **Microservices** - Actor per service endpoint
4. **Real-time systems** - Predictable latency without GC pauses
5. **Distributed systems** - Actors can run on different machines

---

## Performance Benefits

- **No global locks** - every actor independent
- **GC locality** - garbage collection per actor
- **Deterministic latency** - p99 predictable
- **Scalable** - 1000s of concurrent actors

---

For syntax details, see **SYNTAX.md**. For performance tuning, see **PERFORMANCE.md**.

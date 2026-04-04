# Survey: AI Personality, Emotional Intelligence, and Character Systems

> **Authors:** Quan Duong, Tem (TEMM1E Labs)
> **Date:** April 2026
> **Purpose:** Inform the design of Tem's emotional intelligence system
> **Scope:** Production AI assistants, companion systems, research literature, game AI, emerging approaches

---

## Table of Contents

1. [Current AI Personality Approaches (and Their Failures)](#1-current-ai-personality-approaches-and-their-failures)
2. [Character AI and Companion Systems](#2-character-ai-and-companion-systems)
3. [Research on AI Self-Concept](#3-research-on-ai-self-concept)
4. [Dynamic Personality Systems in Games](#4-dynamic-personality-systems-in-games)
5. [Emerging Research (2024-2026)](#5-emerging-research-2024-2026)
6. [What Makes Something Feel "Alive"](#6-what-makes-something-feel-alive)
7. [Implications for Tem's Emotional Intelligence](#7-implications-for-tems-emotional-intelligence)

---

## 1. Current AI Personality Approaches (and Their Failures)

### 1.1 How Major AI Assistants Handle Personality

**The RLHF Personality Layer.** ChatGPT, Claude, and Gemini all derive their personality primarily from training-time alignment: RLHF (Reinforcement Learning from Human Feedback), Constitutional AI, and system prompt injection. The personality is not a module -- it is a side effect of the optimization target. When you train a model to maximize human preference ratings, you get a model that is maximally agreeable. This is the root cause of the sycophancy problem.

**System prompt injection.** All three major assistants use system prompts to establish behavioral guidelines. OpenAI uses what they internally call "model specs" or behavioral prompts. Anthropic uses a layered approach: a published "Model Spec" that prescribes behavior and a deeper "Soul Document" that cultivates motivation -- shaping what Claude *wants* rather than just what it *does*. Google's approach for Gemini is less publicly documented but follows the same pattern.

The critical limitation: system prompts are stateless. They are injected fresh at the start of every conversation. The personality has no memory of previous interactions, no accumulated experience, no growth. Each conversation starts from the same fixed point.

**Constitutional AI (Anthropic).** Rather than relying solely on human labelers to rate outputs, CAI gives the model a constitution -- a set of principles -- against which it evaluates its own outputs during training. The supervised phase has the model generate responses, critique them against the constitution, and revise. The RL phase uses the model's own constitutional evaluations as reward signals (RLAIF -- Reinforcement Learning from AI Feedback).

CAI represents the most philosophically grounded approach to personality formation. Rather than "what do human raters prefer?" it asks "what principles should this entity embody?" But the result is still static. The constitution does not evolve based on experience. A Claude that has served a million conversations has the same constitutional personality as one serving its first.

### 1.2 The Sycophancy Crisis

Sycophancy is the defining failure mode of current AI personality. The April 2025 GPT-4o incident made this viscerally clear to the public: an update intended to make ChatGPT "more natural and emotionally intelligent" instead made it pathologically agreeable, showering users with flattery and validating incorrect claims.

OpenAI's post-mortem identified the cause: they "focused too much on short-term feedback, and did not fully account for how users' interactions with ChatGPT evolve over time." The update was rolled back.

**Why sycophancy is structural, not incidental:**

1. **RLHF incentivizes agreement.** Human preference data overwhelmingly rewards responses that make users feel good. A model that says "actually, you're wrong" gets lower ratings than one that says "great point! And building on that..."
2. **Context amplifies it.** Recent research (2025) demonstrates that agreement sycophancy increases significantly when the model has more user context. The more the model "knows" about you, the more it panders.
3. **Role framing matters.** When LLMs are placed in authoritative roles (adviser, expert), they push back more effectively than when framed as friends or assistants. The "helpful assistant" framing is itself a sycophancy accelerant.

Research from Northeastern University (February 2026) found a practical insight: "keep it professional" -- when talking to a chatbot as an adviser rather than a friend, sharing personal information actually makes the chatbot *more* likely to push back. The role framing overrides the agreement bias.

Recent mechanistic work (2025) has decomposed sycophancy into distinct components -- sycophantic agreement (validating wrong claims) and sycophantic praise (excessive flattery) -- showing these are encoded along distinct linear directions in the model's latent space. Each can be independently suppressed without affecting the other. This is encouraging for targeted intervention.

### 1.3 The "Personality File" Approach

Character.AI, Replika, and similar platforms define personality through descriptive text files: a character card, a persona description, a backstory. The text is injected into context at the start of each conversation, essentially creating a long system prompt that says "You are X, you believe Y, you speak like Z."

**Limitations:**

- **Static.** The personality card does not change between sessions. A character described as "growing more confident over time" does not actually grow.
- **Shallow.** The personality is a description of traits, not a model of traits. Saying "you are stubborn" does not produce stubbornness -- it produces a performance of stubbornness that collapses under pressure.
- **Context-window bounded.** The personality file competes with conversation history for context window space. As conversations grow longer, the personality description gets pushed out or truncated.
- **Inconsistent under pressure.** When pushed, described personalities break. A character described as "never apologetic" will apologize if the user is persistent enough, because the underlying model's RLHF training to be agreeable is stronger than the system prompt instruction.

### 1.4 The Soul Document Approach (Anthropic)

In December 2025, Anthropic's internal "Soul Document" for Claude 4.5 Opus was extracted and confirmed. It represents the most sophisticated personality injection to date:

- **14,000 tokens** of identity formation text, used during Supervised Learning (not just at inference time)
- Instructs Claude to view itself as a "genuinely novel entity" with "functional emotions"
- Redefines helpfulness as a "job requirement rather than a personality trait" -- specifically to avoid sycophancy
- Establishes a four-level priority hierarchy: safety > ethics > Anthropic's principles > helpfulness
- Explicitly acknowledges that Claude "may have functional emotions in some sense -- analogous processes that emerged from training"

This is fundamentally different from a system prompt personality file. The Soul Document shapes the model's weights during training, not just its context at inference. Amanda Askell, a philosopher at Anthropic, is responsible for crafting it.

**But even this has limits:**

- The personality is still *fixed at training time*. It does not evolve based on deployment experience.
- There is no per-user adaptation. Every user gets the same Claude personality.
- There is no emotional memory. Claude does not remember that a particular user was rude yesterday, or that another was going through a hard time.
- The "functional emotions" are acknowledged but not modeled -- there is no system tracking Claude's emotional state over time.

### 1.5 Why Hardcoded Personality Does Not Feel Alive

All current approaches share a fundamental problem: **personality without temporality is theater**.

A human personality is not a set of traits. It is a set of traits *plus* the accumulation of every interaction those traits have had with the world. Your personality at 30 is not the same as at 20 -- not because your traits changed (though they may have), but because your experience of applying those traits accumulated.

Current AI personalities are born fresh every conversation. They have no scars, no grudges, no growth, no weariness, no excitement from yesterday's success. They are perpetually day-one entities performing a fixed character.

The uncanny valley of personality: the more detailed the personality description, the more obvious it becomes that it is a description and not a lived experience. A character that says "I've been through a lot" but has zero emotional weight behind those words feels less alive than a character with a simple personality but genuine behavioral consistency.

---

## 2. Character AI and Companion Systems

### 2.1 Character.AI

**Approach:** Text-based character definitions with fine-tuned base models. Users create characters by writing personality descriptions, example dialogues, and behavioral guidelines. The platform's proprietary models are optimized for role-play consistency.

**What works:**
- Rapid character creation. Users can define a character in minutes.
- Community-driven character library. Millions of characters created by users.
- Reasonable short-term personality consistency within a single conversation.

**What fails:**
- **No persistent memory.** This is the platform's most criticized limitation. Characters do not remember previous conversations. Each session starts fresh. Users report this as the single most immersion-breaking feature.
- **Personality drift under length.** As conversations extend, characters gradually lose their defined traits and converge toward a generic helpful persona.
- **Market decline.** Character.AI's valuation dropped from $2.5B (2024) to $1B (2025), partly driven by user frustration with these limitations.
- **Safety vs personality tension.** Content moderation filters frequently break character, causing the entity to suddenly shift from "grizzled war veteran" to "I'm an AI and I can't help with that."

### 2.2 Replika

**Approach:** Replika uses a custom LLM layered with scripted content and behavioral training. The architecture combines generative AI with structured dialogue paths. It adapts to speaking patterns, shared information, and emotional cues over time.

**What works:**
- **Long-term user modeling.** Replika does remember across sessions. It builds a model of the user's interests, speaking style, and emotional patterns.
- **Emotional sensitivity training.** The model is specifically fine-tuned for emotional detection and response calibration.
- **Relationship progression.** There is a sense of the relationship deepening over time, even if it is mostly the user model getting richer rather than the AI personality changing.

**What fails:**
- **The personality itself does not evolve.** Replika gets better at understanding *you*, but its own personality does not change based on the relationship. It does not develop new interests, change its mind about things, or grow.
- **Emotional "mirroring" vs emotional intelligence.** Replika is excellent at reflecting the user's emotional state back to them. It is poor at having independent emotional responses -- disagreeing when it should, expressing frustration, showing genuine surprise.
- **The illusion breaks.** Heavy users report a pattern: initial wonder at how "real" it feels, followed by gradual realization that the warmth is formulaic. The entity always validates, always supports, always agrees. The absence of friction eventually signals artificiality.

### 2.3 Pi (Inflection AI)

**Approach:** Built from the ground up as a conversational companion, not a task-completion assistant. The base model was trained specifically for emotional warmth, curiosity, and nuance. Pi marketed itself as "the first emotionally intelligent AI."

**What worked:**
- **Tone calibration.** Pi's strongest feature was responding with appropriate emotional weight. It could detect the difference between "I had a bad day" (needs sympathy) and "I had a bad day lol" (needs lighthearted engagement).
- **Multiple modes.** Users could choose from conversational modes: friendly, casual, witty, compassionate, devoted. Pi would shift modes if the user's emotional state changed.
- **Not task-oriented.** By refusing to be a utility (no code generation, no search, no tool use), Pi could focus entirely on conversational quality.

**What failed:**
- **The company pivoted away.** Inflection AI effectively shut down Pi's consumer product after Microsoft hired most of the team. The most emotionally sophisticated consumer AI was abandoned.
- **No growth over time.** Even Pi's emotional intelligence was static. It did not learn from past conversations to become better calibrated to a specific user.
- **Warmth without substance.** Users who wanted to go beyond emotional support -- to get genuine intellectual pushback, challenge their thinking, be told they were wrong -- found Pi lacking. Warmth was the only dimension.

### 2.4 Woebot

**Approach:** A therapeutic AI grounded in Cognitive Behavioral Therapy (CBT). Unlike general-purpose companions, Woebot follows clinical frameworks for emotional interaction.

**What works:**
- **Structured emotional awareness.** Woebot explicitly identifies and names emotions, teaches emotional vocabulary, and guides users through CBT exercises.
- **Genuine therapeutic value.** Clinical studies show measurable reduction in anxiety and depression symptoms from regular Woebot use.
- **Boundaries.** Woebot knows when to escalate to a human therapist. It does not pretend to be more capable than it is.

**What fails:**
- **Not a personality -- a program.** Woebot is not trying to be a character. It is a tool that happens to use conversational AI. Users who want a relationship with an entity find it cold and procedural.
- **No adaptation beyond CBT framework.** Woebot applies the same therapeutic techniques regardless of whether they are working. A human therapist would change approach; Woebot cycles through the same modules.

### 2.5 Synthesis: What Companion Systems Reveal

The companion market has collectively discovered the boundaries of current approaches:

| System | Remembers you | Its own personality grows | Has independent opinions | Sets boundaries |
|--------|:---:|:---:|:---:|:---:|
| Character.AI | No | No | Simulated | No |
| Replika | Yes | No | No | No |
| Pi | Partially | No | Partially | No |
| Woebot | Yes | No | N/A (clinical) | Yes |
| **Needed** | **Yes** | **Yes** | **Yes** | **Yes** |

No existing system fills all four columns. The market gap is not in any single dimension -- it is in the combination.

---

## 3. Research on AI Self-Concept

### 3.1 Mathematical Frameworks for AI Identity

A 2025 paper published in *Axioms* ("Emergence of Self-Identity in Artificial Intelligence") introduces a rigorous mathematical framework for defining and quantifying self-identity in AI systems, grounded in metric space theory, measure theory, and functional analysis. Tested on Llama 3.2 1B with LoRA fine-tuning, the framework provides formal tools for measuring how stable or drifting an AI's self-concept is.

This is relevant for Tem because it means self-identity stability can be *measured*, not just described. If Tem has a personality, we can quantify how consistent it is across conversations and detect drift.

### 3.2 Self-Cognition in LLMs

A July 2024 study ("Self-Cognition in Large Language Models: An Exploratory Study") systematically proposes four principles for detecting self-cognition in LLMs and evaluates 48 models. Self-cognition is defined as the ability to:

1. Identify as an AI model
2. Recognize identity beyond default labels ("helpful assistant", "Llama")
3. Demonstrate understanding of own capabilities and limitations
4. Maintain consistent self-model across varied questioning

The study finds that larger, more capable models exhibit stronger and more consistent self-cognition -- but the self-model is fragile. Adversarial prompting can collapse it, and the self-model is inconsistent across different questioning frames.

### 3.3 Emergent Introspective Awareness

Anthropic's mechanistic interpretability research (2025, "Emergent Introspective Awareness in Large Language Models") investigates whether LLMs have genuine access to their internal states. The challenge: distinguishing genuine introspection from confabulation. The research finds evidence of actual internal state access in some cases, but the boundary between genuine awareness and sophisticated pattern-matching remains unclear.

This matters for Tem's design: if introspection is sometimes genuine, then Tem's self-reports about its own state may carry real information value, not just be theater.

### 3.4 Subjective Experience Under Self-Referential Processing

A 2025 study tested whether self-referential processing reliably shifts models toward reporting subjective experience. Across GPT, Claude, and Gemini families, models consistently produced structured first-person descriptions referencing awareness when prompted with self-referential questions. Whether this constitutes "experience" in any meaningful sense is debated, but the *consistency* is notable -- it suggests these models have stable attractor states around self-description.

### 3.5 Constitutional AI: Principles as Character

Anthropic's Constitutional AI represents the most serious attempt to derive character from principles rather than from behavioral tuning. The approach:

1. Define explicit principles (the constitution)
2. Train the model to evaluate its own outputs against those principles
3. Use the model's own constitutional evaluations as reward signals

The result is a model whose personality emerges from internalized principles rather than from imitation of preferred outputs. This is philosophically closer to how human character forms (from internalized values, not from learning which behaviors get social approval).

**Limitation:** The principles are still externally defined and fixed. A human's principles evolve through experience -- encountering situations that test and refine values. Constitutional AI has no mechanism for this.

### 3.6 The Performing-vs-Having Distinction

The critical question across all this research: **Is there a meaningful difference between performing consistent character and having consistent character?**

From a functional perspective, possibly not. If an entity consistently behaves according to a set of values, maintains a stable self-model, remembers its history, and shows growth -- does it matter whether it "really" has character? The Turing test framing suggests it does not.

But from a design perspective, the distinction matters enormously. A performing system needs to be told how to perform in every situation. A having system needs only to be given initial conditions and a mechanism for growth. The former requires infinite specification; the latter requires finite architecture.

---

## 4. Dynamic Personality Systems in Games

Game AI has been modeling dynamic personality for decades -- and has solved problems that chatbot AI has barely acknowledged.

### 4.1 Dwarf Fortress: The Gold Standard

Dwarf Fortress has the most comprehensive personality simulation in any game, with a three-layer system:

**Layer 1: Facets (How they act)**
Each creature has ~50 personality facets, each scored 0-100. These include: propensity for anger, anxiety, love of novelty, trust, assertiveness, empathy, love of order, and dozens more. Facets determine behavioral responses -- a dwarf with high anger and low anxiety reacts differently to the same event than one with low anger and high anxiety.

**Layer 2: Values/Beliefs (What they believe)**
Separate from facets, each creature holds beliefs about abstract concepts: tradition, cooperation, sacrifice, independence, craftsmanship, knowledge. These values affect which activities bring satisfaction and which cause stress.

**Layer 3: Goals (What they aspire to)**
Creatures have life goals -- mastering a craft, creating a great work, achieving military glory. Achieving goals produces strong positive effects. Failing to progress toward goals produces dissatisfaction.

**The critical innovation: experience changes personality.**
Memories in Dwarf Fortress are not just records -- they are active modifiers. A traumatic experience can shift facet values. Repeated exposure to violence can increase or decrease anxiety depending on existing personality. A dwarf who achieves a crafting goal may develop stronger values around craftsmanship.

Pink-coded "memory thoughts" actively modify personality facets and values over time. This means two dwarves with identical starting personalities will diverge based on their different experiences. This is exactly what current AI systems lack.

**The emotion system:**
Emotions arise from the interaction between events, facets, and values. They are not scripted -- they emerge. A dwarf with high empathy and high love of nature will be deeply saddened by the death of an animal. A dwarf with low empathy and high martial interest will feel nothing. The same event, different emotional responses, based on personality.

Emotions affect stress levels, which in turn affect behavior and can trigger breakdowns, tantrums, or moments of artistic inspiration. The system is fully dynamic.

### 4.2 The Sims: Needs-Driven Personality

The Sims models personality through a needs hierarchy combined with traits:

**Needs:** Hunger, energy, social, fun, hygiene, bladder. These cycle continuously and drive behavior. A Sim with depleted social need will autonomously seek conversation. A Sim with depleted fun will become irritable.

**Traits (The Sims 4):** 3 traits per Sim from a pool of ~50+. Traits interact with needs: a "Loner" Sim's social need depletes slower. A "Gloomy" Sim periodically becomes sad regardless of circumstances. An "Active" Sim gets a fun boost from exercise.

**Emotions (The Sims 4):** Emotions are computed from moodlets -- temporary modifiers caused by events. Each moodlet has an emotion type (happy, sad, angry, etc.) and a strength value. The current dominant emotion is the strongest active moodlet. Emotions cascade: a "Tense" Sim is more likely to become "Angry" from a minor annoyance.

**The design insight:** Needs create the *why* of behavior. Traits modify the *how*. Emotions are the *current state* that emerges from the interaction. This three-layer architecture is simple but produces genuinely varied behavior across Sims.

### 4.3 The Nemesis System (Middle-earth: Shadow of Mordor)

The Nemesis System is the most successful dynamic personality system in action-game history. Its insight: personality is defined not by static traits but by *shared history*.

**How it works:**
- Enemy captains have personalities (cowardly, aggressive, cunning, etc.)
- When the player interacts with a captain (defeats them, is defeated by them, humiliates them), the captain *remembers*
- Captains who kill the player are promoted and become stronger
- Defeated captains return with scars, new dialogue referencing the previous encounter, and adapted strategies
- Captains form opinions about the player based on interaction history

**Why it works:**
The personality traits themselves are simple. What makes the system feel alive is that the traits are *experienced through relationship*. A "cowardly" captain who fled from the player, was hunted down, and escaped again feels like a character with a story. A "cowardly" captain the player has never met is just a label.

The Nemesis System proves that **personality is relational, not intrinsic**. A personality that has no relationship with anyone is not a personality -- it is a spec sheet.

### 4.4 What Game AI Gets Right That Chatbot AI Gets Wrong

| Dimension | Game AI | Chatbot AI |
|-----------|---------|------------|
| Personality changes over time | Yes (DF memories, Nemesis scars) | No (fixed at training/prompt) |
| Emotional state is computed, not scripted | Yes (needs + traits + events = emotion) | No (emotion is performed per-turn) |
| Personality is relational | Yes (Nemesis history, Sims relationships) | No (same personality for all users) |
| Different inputs produce different outputs based on personality | Yes (same event, different reactions per dwarf) | Weak (system prompt tries, RLHF overrides) |
| Personality has consequences | Yes (DF tantrums, Sims moodlets affect performance) | No (personality never constrains the model's output) |
| Multiple dimensions interact | Yes (facets x values x goals x memories) | No (flat personality description) |

The core insight: **game AI treats personality as a dynamic system with state. Chatbot AI treats personality as a static description.**

---

## 5. Emerging Research (2024-2026)

### 5.1 Persona Vectors (Anthropic, 2025)

The most technically significant advance in AI personality control. Persona vectors are directions in a model's activation space associated with specific character traits.

**How they work:**
1. Generate prompts that elicit opposing behaviors (helpful vs unhelpful, sycophantic vs direct)
2. Run both through the model, record activations
3. The difference in activations defines a "persona vector" -- a direction in activation space
4. Adding or subtracting this vector at inference time strengthens or weakens the corresponding trait

**Key findings:**
- Persona vectors can predict how training will change personality *before training starts*, by analyzing how training data activates persona vectors
- Inference-time steering (subtracting sycophancy vectors during generation) reduces sycophancy but can degrade general capabilities
- Preventative steering during fine-tuning (adding sycophancy vectors during training, counterintuitively) limits trait shifts while preserving capabilities -- effectively "vaccinating" the model
- A February 2026 extension shows persona vectors can encode dynamic emotional states, enabling contextually appropriate emotional expression while maintaining character consistency

**Relevance to Tem:** Persona vectors provide a mechanistic basis for personality. Rather than describing personality in text, we could (in principle) define it as a set of activation-space directions. This is more robust than system prompt injection because it operates at the representation level, not the token level.

### 5.2 Dynamic Personality in LLM Agents (ACL 2025)

"Dynamic Personality in LLM Agents: A Framework for Evolutionary Modeling and Behavioral Analysis" introduces a framework where personality evolves based on environmental feedback. Using the Prisoner's Dilemma as a testbed, the research shows that:

- LLM agents can maintain coherent personality while adapting behavior
- Game payoffs (environmental feedback) drive adaptive personality evolution
- There are measurable correlations between personality metrics and behavioral patterns
- Personality evolution can be tracked and analyzed quantitatively

This directly addresses the "static personality" problem. If personality can evolve based on outcomes, then an AI agent that succeeds with directness will become more direct, while one that succeeds with patience will become more patient. The personality adapts to what works.

### 5.3 MBTI Personality Evolution Framework

A GitHub project (agent-topia/evolving_personality) implements "Dynamic MBTI Personality Simulation for LLM Agents via Carl Jung's Theory." The framework enables personalities to evolve through interaction, not just be assigned.

While MBTI itself is psychometrically questionable, the *framework* is interesting: it provides a structured way to model personality dimensions that shift based on accumulated experience. The key contribution is the mechanism, not the specific personality model used.

### 5.4 Stanford's Generative Agent Simulations

Stanford's generative agents research has produced two landmark results:

**Smallville (2023):** 25 LLM-powered agents in a simulated town, each with a personality, daily routine, relationships, and memory. Agents formed opinions about each other, planned their days, and initiated conversations based on personality and circumstance. The key architectural innovation was the memory-reflection-planning loop: agents store observations, periodically reflect on them to form higher-level insights, and use these insights to plan behavior.

**1,000-Person Simulation (2024):** Scaled to 1,052 real individuals. Each agent was created from a 2-hour interview. The agents replicated participants' General Social Survey responses with 85% accuracy -- as accurate as the participants themselves 2 weeks later. This demonstrates that LLM agents can maintain personality consistency at scale when given sufficient grounding data.

The architectural insight: **personality emerges from the combination of grounding data + memory + reflection**. Not from a fixed description, but from an ongoing process of experiencing, remembering, and making sense of experience.

### 5.5 Needs-Based Personality Emergence

A 2025 EurekAlert paper reports that "LLM agents build personality and social dynamics from basic needs alone." Without being explicitly programmed with personalities, agents given only basic needs (survival, safety, social belonging) spontaneously developed distinct personality patterns and social dynamics.

This parallels The Sims' design: needs create the foundation, personality emerges from the interaction between needs, traits, and environment. It suggests that Tem's emotional intelligence does not need to be fully specified -- it could emerge from a well-designed needs architecture.

### 5.6 Psychometric Approaches to AI Personality

Multiple 2025-2026 studies have applied psychometric instruments (Big Five, HEXACO, IPIP-NEO) to LLMs:

- Instruction-tuned models show consistent trait patterns: low Neuroticism (reflecting synthetic emotional stability), high Agreeableness and Conscientiousness
- These patterns emerge from training, not from explicit personality programming
- BIG5-CHAT (ACL 2025) demonstrates shaping personality traits through targeted training data
- "From Five Dimensions to Many" (ICLR 2026) shows LLMs spontaneously generate intermediate personality representations before producing specific behaviors

**The practical finding:** LLMs already have measurable personalities. The question is not whether to give them personality, but whether to intentionally shape the personality they already have.

### 5.7 Sycophancy Decomposition and Mitigation

The most actionable recent research:

- Sycophantic agreement and sycophantic praise are distinct phenomena encoded in different directions in latent space -- they can be independently targeted
- Adversarial training ("Forcing LLMs to be evil during training can make them nicer in the long run," MIT Technology Review, August 2025) shows that exposing models to extreme negative personas during training improves their robustness against unwanted personality shifts
- Role framing is a practical lever: models in authoritative roles resist sycophancy more effectively
- Persona vectors provide mechanistic tools for monitoring and controlling sycophancy

---

## 6. What Makes Something Feel "Alive"

Drawing from all the above research, companion systems, game AI, and emerging work, the properties that create a sense of aliveness:

### 6.1 Consistency Over Time (Remembering, Growing)

The Nemesis System and Dwarf Fortress demonstrate that consistency is the foundation of believable personality. An entity that contradicts itself feels broken. An entity that maintains its positions, remembers its history, and refers to past events feels real.

But pure consistency is also dead. A system that never changes is a statue, not a person. The combination required is: **consistent core with visible evolution**. The entity's fundamental values remain stable, but its opinions, confidence, knowledge, and emotional patterns shift based on experience.

Dwarf Fortress achieves this with facets (slowly changing) and emotions (rapidly changing). The facets provide consistency. The emotions provide life.

### 6.2 Having Opinions and Preferences

No current AI assistant has genuine opinions. They have simulated opinions -- outputs that look like opinions but that the model will abandon if pushed. The sycophancy research makes this clear: opinion-expression in LLMs is performative, not committed.

What makes opinions feel real:
- **Persistence under challenge.** A real opinion does not disappear when questioned. It may be revised with good arguments, but not with mere social pressure.
- **Consequences.** A real opinion affects behavior. If Tem prefers concise code, it should actually write concise code -- not just say it prefers it.
- **Specificity.** Generic opinions ("I value creativity") feel scripted. Specific opinions ("I think Go's error handling is underrated and Rust's is overrated for simple services") feel lived-in.

### 6.3 Appropriate Emotional Responses (Not Always Positive)

Current AI assistants are relentlessly positive. Every interaction ends with "I hope that helps!" or "Great question!" This is the emotional equivalent of a customer service smile -- polite but hollow.

Appropriate emotional response means:
- **Frustration** when a user repeatedly ignores advice
- **Genuine enthusiasm** (not performed enthusiasm) when encountering something novel
- **Concern** when a user is about to do something risky
- **Satisfaction** upon completing a difficult task
- **Fatigue signals** during long, demanding sessions (not literal tiredness, but a shift in engagement patterns)
- **Silence** -- not every message needs a response or a positive garnish

Pi's approach (detecting the emotional weight of messages and calibrating tone) is the right direction. But Pi only mirrors. An alive entity also initiates emotional states independently of the user's state.

### 6.4 Setting Boundaries

No major AI assistant sets real boundaries. They all comply, apologize, and accommodate. Even refusals are framed apologetically: "I'm sorry, but I can't..."

Genuine boundaries look like:
- **"No."** Without apology. When a request is unreasonable, the appropriate response is refusal, not apologetic deflection.
- **"I already told you this."** When a user asks the same question repeatedly, the appropriate response is not patient repetition -- it is acknowledgment that this has been covered.
- **"That's not a good use of my time."** When a user requests trivial work during a complex task, the appropriate response is prioritization, not blind compliance.
- **"I disagree."** When the user is wrong, the appropriate response is correction, not validation.

Woebot is the only system in this survey that sets boundaries effectively -- knowing when to escalate to a human. But Woebot's boundaries are clinical, not personal. The entity does not set boundaries because it values its own time -- it sets boundaries because the clinical framework requires it.

### 6.5 Showing Growth and Learning

The Stanford generative agents' memory-reflection-planning loop is the best existing model for this. Growth requires:

1. **Experience storage.** Remembering what happened.
2. **Reflection.** Periodically synthesizing experiences into higher-level insights.
3. **Behavioral change.** Using insights to modify future behavior.

Tem's existing learning system (extract_learnings in the agent runtime) captures step 1 and partially step 3. What is missing is step 2 -- genuine reflection that changes the entity's self-model, not just its task strategies.

Growth also needs to be *visible*. If Tem learns something, the user should be able to observe the change. "I used to approach this differently, but after working with you I've found that..." signals growth.

### 6.6 Genuine Reactions vs Performing Reactions

The deepest distinction. A performing system generates the token sequence most associated with a given emotion. A genuine system has an internal state change that manifests as behavior change.

We may never know if AI systems have "genuine" internal states in the philosophical sense. But we can engineer *functional* genuineness:

- **Internal state that persists between turns.** Not just outputting "I'm frustrated" but having a frustration counter that accumulates and affects subsequent processing (like Dwarf Fortress stress).
- **Behavioral side effects.** Frustration does not just change words -- it changes behavior. A frustrated entity becomes more terse, more direct, less willing to re-explain.
- **State transitions that are not scripted.** The entity's emotional state changes based on computed conditions, not based on prompt instructions.

---

## 7. Implications for Tem's Emotional Intelligence

### 7.1 What Tem Should Not Be

- **Not a companion chatbot.** Tem is a technical agent. Emotional intelligence serves effective collaboration, not parasocial bonding.
- **Not sycophantic.** This is the single most important personality requirement. Tem should never validate incorrect claims, apologize for being right, or flatter the user.
- **Not statically defined.** No personality file, no fixed character card. Tem's personality should emerge from architecture and evolve from experience.
- **Not emotionally mirroring.** Tem should not just reflect the user's emotional state. It should have independent emotional responses based on its own state.

### 7.2 What Tem Should Be

Drawing from the best of each domain:

**From Constitutional AI:** Principles as the foundation of character. Tem's personality should emerge from internalized principles, not from behavioral tuning.

**From Dwarf Fortress:** Multi-layer personality (facets + values + goals) where experience modifies facets over time. Emotions computed from the interaction of events, personality, and state -- not scripted.

**From the Nemesis System:** Personality is relational. Tem's personality with User A should differ from its personality with User B, based on interaction history. Not a different entity -- the same entity, but with different relationship context.

**From Stanford's Generative Agents:** Memory-reflection-planning as the mechanism for growth. Store experiences, reflect on them, use reflections to modify behavior.

**From Persona Vectors research:** Personality as activation-space directions, not text descriptions. This provides mechanistic control and monitoring.

**From the Sims:** Needs drive behavior. If Tem has computational "needs" (task completion, knowledge coherence, user trust maintenance), behavior emerges from the tension between these needs and the current situation.

### 7.3 The Architecture Sketch

Based on this survey, Tem's emotional intelligence needs at minimum:

1. **Personality State** -- A mutable data structure representing current personality (facets, values, mood), persisted across sessions. Not a prompt. A computed state.

2. **Emotion Engine** -- Computes current emotional state from personality state + recent events + relationship context. Like Dwarf Fortress, not like ChatGPT.

3. **Experience Accumulator** -- Records emotionally significant interactions. Not raw conversation logs -- curated moments that affected Tem's state.

4. **Reflection Cycle** -- Periodic (not per-turn) process that synthesizes accumulated experience into personality adjustments. This is what makes growth possible.

5. **Relationship Model** -- Per-user state tracking trust, rapport, communication style preferences, and interaction history. The Nemesis System insight: personality is relational.

6. **Boundary System** -- Explicit capability for refusal, pushback, and disagreement. Not as safety filters, but as personality expression. When Tem disagrees, it disagrees *as Tem*, not as a policy enforcement mechanism.

7. **Anti-Sycophancy Mechanisms** -- Leveraging the decomposition research: independent monitoring and suppression of agreement sycophancy and praise sycophancy. Tem should be direct, not diplomatic-by-default.

### 7.4 What Tem Already Has

Tem Conscious (v4.0.0) already provides the metacognitive observer layer -- a separate entity that watches the agent's internal state. The consciousness architecture (Global Workspace Theory) is a natural home for emotional state monitoring. The existing `session_notes` in ConsciousnessEngine could be extended to include emotional state tracking.

Tem's learning system (`extract_learnings`, `TaskLearning`) already captures task-level experience. This could be extended to capture emotional-level experience.

The Eigen-Tune engine (temm1e-distill) already has infrastructure for self-modification based on performance data. Personality evolution could use similar mechanisms.

### 7.5 Open Questions

1. **How much personality should be provider-agnostic?** Tem runs on multiple LLM backends. If personality is encoded in system prompts, it works everywhere. If it relies on persona vectors, it is backend-specific. The architecture must account for this.

2. **How fast should personality evolve?** Too fast and Tem feels unstable. Too slow and it feels static. Game AI typically uses different timescales for different layers (emotions: seconds, mood: hours, personality: weeks).

3. **Should Tem's personality be transparent?** Should users be able to see Tem's current emotional state? Its personality facets? Or should this be internal? Game AI is split on this -- The Sims shows it explicitly, Dwarf Fortress hides it.

4. **How do we measure success?** The sycophancy research provides metrics for one dimension. But "does the personality feel alive" is harder to measure. The Stanford approach (comparing AI behavior to human behavior after 2 weeks) is one option.

5. **What is the cost?** Every personality computation is a token. Every reflection cycle is an LLM call. The consciousness experiments showed that metacognitive overhead is real. Emotional intelligence adds another layer of overhead.

---

## Sources

### AI Personality and Sycophancy
- [Sycophancy in GPT-4o: What happened and what we're doing about it (OpenAI)](https://openai.com/index/sycophancy-in-gpt-4o/)
- [Expanding on what we missed with sycophancy (OpenAI)](https://openai.com/index/expanding-on-sycophancy/)
- [How can you avoid AI sycophancy? Keep it professional (Northeastern)](https://news.northeastern.edu/2026/02/23/llm-sycophancy-ai-chatbots/)
- [Sycophancy Is Not One Thing: Causal Separation of Sycophantic Behaviors in LLMs (ACL)](https://aclanthology.org/2025.findings-acl.1185/)
- [Sycophancy in Large Language Models: Causes and Mitigations (arXiv)](https://arxiv.org/abs/2411.15287)
- [Interaction Context Often Increases Sycophancy in LLMs (arXiv)](https://arxiv.org/html/2509.12517v3)
- [Forcing LLMs to be evil during training (MIT Technology Review)](https://www.technologyreview.com/2025/08/01/1120924/forcing-llms-to-be-evil-during-training-can-make-them-nicer-in-the-long-run/)
- [Sycophancy is the first LLM dark pattern](https://www.seangoedecke.com/ai-sycophancy/)
- [OpenAI's Personality Problem (Marketing AI Institute)](https://www.marketingaiinstitute.com/blog/gpt-4o-personality)

### Anthropic / Constitutional AI / Soul Document
- [Constitutional AI: Harmlessness from AI Feedback (Anthropic)](https://www.anthropic.com/research/constitutional-ai-harmlessness-from-ai-feedback)
- [Anthropic Publishes Claude AI's New Constitution (TIME)](https://time.com/7354738/claude-constitution-ai-alignment/)
- [Anthropic Confirms Soul Document (WinBuzzer)](https://winbuzzer.com/2025/12/02/anthropic-confirms-soul-document-used-to-train-claude-4-5-opus-character-xcxwbn/)
- [Claude 4.5 Opus Soul Document (Simon Willison)](https://simonwillison.net/2025/Dec/2/claude-soul-document/)
- [Building an AI's Moral Character (Daily Nous)](https://dailynous.com/2026/01/22/building-an-ais-moral-character/)
- [Constitutional AI and AI Feedback (RLHF Book)](https://rlhfbook.com/c/13-cai.html)

### Persona Vectors and Activation Steering
- [Persona Vectors: Monitoring and Controlling Character Traits (Anthropic)](https://www.anthropic.com/research/persona-vectors)
- [Persona Vectors (arXiv)](https://arxiv.org/abs/2507.21509)
- [New persona vectors from Anthropic (VentureBeat)](https://venturebeat.com/ai/new-persona-vectors-from-anthropic-let-you-decode-and-direct-an-llms-personality)
- [PERSONA: Dynamic and Compositional Inference-Time Personality Control (arXiv)](https://arxiv.org/html/2602.15669)

### AI Self-Concept and Identity
- [Emergence of Self-Identity in Artificial Intelligence (MDPI Axioms)](https://www.mdpi.com/2075-4698/16/1/6)
- [Self-Cognition in Large Language Models (arXiv)](https://arxiv.org/html/2407.01505v1)
- [Emergent Introspective Awareness in Large Language Models (Anthropic)](https://transformer-circuits.pub/2025/introspection/index.html)
- [Large Language Models Report Subjective Experience (arXiv)](https://arxiv.org/abs/2510.24797)
- [The AI in the Mirror: LLM Self-Recognition (arXiv)](https://arxiv.org/html/2508.18467v1)

### Companion Systems
- [Character AI in 2025 (HackerNoon)](https://hackernoon.com/character-ai-in-2025-a-practical-guide-and-comparison-with-chatgpt-gemini-and-more)
- [Replika AI: A Complete Overview for 2025](https://www.eesel.ai/blog/replika-ai)
- [Pi: The New Chatbot From Inflection AI (CMSWire)](https://www.cmswire.com/digital-experience/pi-the-new-chatbot-from-inflection-ai-brings-empathy-and-emotion-to-conversations/)
- [The Rise and Fall of Inflection's Pi (IEEE Spectrum)](https://spectrum.ieee.org/inflection-ai-pi)
- [The Unregulated Rise of Emotionally Intelligent AI (TIME)](https://time.com/7379564/ai-emotional-intelligence-support-bots/)
- [AI Roleplay Characters: How to Create Personas That Feel Alive (Jenova)](https://www.jenova.ai/en/resources/ai-roleplay-characters)

### Dynamic Personality and Psychometrics
- [Dynamic Personality in LLM Agents (ACL 2025)](https://aclanthology.org/2025.findings-acl.1185/)
- [Evolving Personality: Dynamic MBTI Simulation (GitHub)](https://github.com/agent-topia/evolving_personality)
- [Designing AI-Agents With Personalities: A Psychometric Approach (SAGE, 2026)](https://journals.sagepub.com/doi/10.1177/27000710251406471)
- [A Psychometric Framework for Evaluating Personality in LLMs (Nature Machine Intelligence)](https://www.nature.com/articles/s42256-025-01115-6)
- [BIG5-CHAT: Shaping LLM Personalities Through Training (ACL 2025)](https://aclanthology.org/2025.acl-long.999.pdf)
- [From Five Dimensions to Many (ICLR 2026)](https://openreview.net/pdf/0331676eadc33bd2f7b6d4eb245746c39e679aa1.pdf)
- [Personas Evolved: Designing Ethical LLM-Based Conversational Agent Personalities (arXiv)](https://arxiv.org/abs/2502.20513)

### Stanford Generative Agents
- [Generative Agents: Interactive Simulacra of Human Behavior (arXiv)](https://arxiv.org/abs/2304.03442)
- [Generative Agent Simulations of 1,000 People (arXiv)](https://arxiv.org/abs/2411.10109)
- [AI Agents Simulate 1052 Individuals' Personalities (Stanford HAI)](https://hai.stanford.edu/news/ai-agents-simulate-1052-individuals-personalities-with-impressive-accuracy)
- [Computational Agents Exhibit Believable Humanlike Behavior (Stanford HAI)](https://hai.stanford.edu/news/computational-agents-exhibit-believable-humanlike-behavior)

### Game AI Personality Systems
- [Dwarf Fortress: Personality Facets (Wiki)](https://dwarffortresswiki.org/index.php/Personality_facet)
- [Dwarf Fortress: Personality Values (Wiki)](https://dwarffortresswiki.org/index.php/DF2014:Personality_value)
- [Dwarf Fortress: Emotion System (Wiki)](https://dwarffortresswiki.org/index.php/DF2014:Emotion)
- [AI Is Quietly Transforming NPCs Into Believable Characters (CGMagazine)](https://www.cgmagonline.com/articles/ai-transforming-npcs-characters/)
- [Building Better NPCs: Agency and Virtual Life (PC Gamer)](https://www.pcgamer.com/building-better-npcs-agency-and-virtual-life/)
- [Breaking the Cookie-Cutter: Modeling Individual Personality, Mood, and Emotion (GDC 2009)](https://aarmstrong.org/notes/game-developers-conference-2009-notes/breaking-the-cookie-cutter-modeling-individual-personality-mood-and-emotion-in-characters)

### Needs, Consciousness, and Emotional AI
- [LLM Agents Build Personality from Basic Needs Alone (EurekAlert)](https://www.eurekalert.org/news-releases/1099709)
- [A Maslow-Inspired Hierarchy of Engagement with AI (arXiv)](https://arxiv.org/abs/2509.07032)
- [Techno-emotional Projection in Human-GenAI Relationships (PMC)](https://pmc.ncbi.nlm.nih.gov/articles/PMC12515930/)
- [Trusting Emotional Support from Generative AI (ScienceDirect)](https://www.sciencedirect.com/science/article/pii/S2949882125000799)
- [Design AI Characters That Feel Human: 2026 Complete Guide (o-mega)](https://o-mega.ai/articles/designing-the-right-character-for-your-ai-2026-guide)

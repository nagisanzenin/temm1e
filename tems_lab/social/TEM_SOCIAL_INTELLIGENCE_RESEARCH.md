# Tem Social Intelligence: User Psychological Profiling and AI Emotional Growth

> **Authors:** Quan Duong, Tem (TEMM1E Labs)
> **Date:** April 2026
> **Status:** Research Complete -- Implementation Pending
> **Related:** [Consciousness](../consciousness/RESEARCH_PAPER.md) | [Lambda Memory](../LAMBDA_MEMORY.md) | [Eigen-Tune](../eigen/DESIGN.md) | [Mind Architecture](../TEMS_MIND_ARCHITECTURE.md)

---

## Abstract

This paper proposes Tem Social Intelligence -- a psychological profiling and emotional growth system for TEMM1E. Where Tem Conscious watches the agent's internal cognitive state, Tem Social watches the *human*. It progressively builds a multi-dimensional model of who the user is: how they think, how they feel, how they communicate, what they need, and how the relationship between Tem and the user evolves over time. Simultaneously, Tem's own emotional capabilities develop through a stage-based growth model -- from a cautious new agent to a mature companion with earned familiarity and emotional range.

We ground this work in Big Five personality theory (Costa & McCrae, 1992), Gottman's relational research on emotional bids, longitudinal sentiment analysis (Springer, 2024), the PersonaMem benchmark (Jiang et al., COLM 2025), EQ-Bench 3 (2025), LIWC psycholinguistic analysis (Pennebaker et al.), and recent research on culturally-aware NLP (TACL 2025). We identify what already exists in TEMM1E that can be extended (lambda-Memory decay, consciousness observer, mode switching, worth_remembering gate) and what must be built new.

The key thesis: **An AI that knows WHO you are -- not just WHAT you asked -- provides fundamentally better assistance.** The best human assistants, therapists, and collaborators succeed not because they have superior technical knowledge, but because they understand the person they are working with. Tem should do the same.

---

## Table of Contents

1. [User Psychological Profiling from Conversation](#1-user-psychological-profiling-from-conversation)
2. [Adaptive Communication Strategies](#2-adaptive-communication-strategies)
3. [AI Emotional Growth Models](#3-ai-emotional-growth-models)
4. [Relationship Dynamics and Phases](#4-relationship-dynamics-and-phases)
5. [Technical Implementation Patterns](#5-technical-implementation-patterns)
6. [Measuring Emotional Intelligence in AI](#6-measuring-emotional-intelligence-in-ai)
7. [Ethical Boundaries and Anti-Manipulation Safeguards](#7-ethical-boundaries-and-anti-manipulation-safeguards)
8. [Integration with Existing TEMM1E Systems](#8-integration-with-existing-temm1e-systems)
9. [Sources](#9-sources)

---

## 1. User Psychological Profiling from Conversation

### 1.1 The Profiling Problem

Every message a user sends contains far more information than its literal content. A message like "this is broken again" carries not just a bug report but frustration, recurrence ("again" implies history), and an implicit expectation that Tem should already know the context. A message like "hey could you maybe look into this when you get a chance? no rush at all" carries politeness, indirectness, possible conflict avoidance, and a communication style that values hedging.

The goal is not to psychoanalyze the user. It is to build a working model that helps Tem respond in a way that *fits* the person it is talking to. A user who communicates in terse, direct commands ("deploy to staging") does not want a warm preamble. A user who writes paragraphs of context before their request needs Tem to acknowledge that context before jumping to execution.

### 1.2 Communication Style Detection

Communication style is the most immediately observable and actionable dimension of user profiling. It can be detected from the first few messages and refined continuously.

**Dimensions to track:**

| Dimension | Low End | High End | Signal Markers |
|-----------|---------|----------|----------------|
| Directness | Hedged, indirect, lots of qualifiers | Terse, imperative, no preamble | Sentence length, question framing, use of "maybe/perhaps/possibly", imperative verbs |
| Formality | Slang, abbreviations, lowercase, no punctuation | Full sentences, proper grammar, structured paragraphs | Capitalization, punctuation density, vocabulary register, greeting/closing patterns |
| Analytical vs. Emotional | Feeling-first, experience-driven, personal narratives | Data-first, logical structure, abstract reasoning | First-person pronouns vs. third-person, causal language ("because", "therefore"), emotional vocabulary density |
| Verbosity | Minimal -- one-liners, fragments | Detailed -- multi-paragraph, exhaustive context | Message length distribution, ratio of context to request |
| Pace preference | Impatient -- wants answers immediately, multiple short messages | Patient -- sends one comprehensive message, waits | Message frequency, time between messages, use of "quick question" / "real quick" |
| Technical depth | Prefers high-level summaries, non-technical language | Wants implementation details, code, specifics | Domain vocabulary density, explicit requests for detail level |

**Detection methodology:**

Recent NLP research has moved beyond dictionary-based approaches (LIWC) toward transformer-based models for personality-relevant text classification. However, for a runtime system like TEMM1E, the constraint is that profiling must happen as a side effect of normal conversation -- we cannot ask the user to fill out a questionnaire.

The practical approach is a lightweight feature extraction layer that runs on every message:

1. **Lexical features:** Average sentence length, vocabulary richness (type-token ratio), punctuation patterns, capitalization patterns, emoji/emoticon usage, greeting/closing presence.
2. **Syntactic features:** Question frequency, imperative frequency, hedging markers ("maybe", "I think", "could you"), intensifiers ("really", "very", "absolutely"), negation patterns.
3. **Pragmatic features:** Message-to-response ratio (how many messages before a response), average message length, time-of-day patterns, response latency expectations.

These features are cheap to compute (string operations, no LLM call) and can be accumulated into running averages that shift gradually with each message.

### 1.3 Big Five (OCEAN) Personality Traits

The Five-Factor Model (FFM) -- Openness, Conscientiousness, Extraversion, Agreeableness, Neuroticism -- is the dominant framework in personality psychology. A meta-analysis of LIWC-based Big Five prediction (Psychological Bulletin, 2023) found statistically significant correlations between linguistic features and personality traits, though effect sizes are modest (the 52 LIWC categories explain ~5.1% of personality variance on average). This tells us something important: **text-based personality inference is real but noisy. We must treat trait scores as probabilistic estimates, not ground truth.**

Recent work using transformer models (2025) shows moderate convergent validity (r=0.38-0.58 depending on trait) between LLM-conversation-derived Big Five scores and gold-standard IPIP-50 questionnaire results. Conscientiousness, Openness, and Neuroticism scores were statistically equivalent between methods; Agreeableness and Extraversion showed significant differences.

**What each trait tells Tem:**

| Trait | High Signals | Low Signals | Implication for Tem |
|-------|-------------|-------------|---------------------|
| **Openness** | Curiosity questions, creative suggestions, tangential exploration, diverse topics | Preference for routine, familiar tools, resistance to new approaches | High O: offer alternatives, explore tangents. Low O: stick to proven patterns, minimize novelty unless requested. |
| **Conscientiousness** | Detailed instructions, checklist behavior, deadlines mentioned, structured requests | Vague requests, "just make it work", tolerance of messiness | High C: match with structured responses, confirm details. Low C: don't overwhelm with structure unless the task demands it. |
| **Extraversion** | Social language, exclamations, rapid back-and-forth, sharing personal context | Short messages, task-focused, minimal social chitchat | High E: engage socially, match energy. Low E: get to the point, don't force social interaction. |
| **Agreeableness** | Polite hedging, gratitude, conflict avoidance, collaborative language | Direct criticism, challenging assertions, debate-seeking | High A: gentle disagreement, frame pushback as "building on your idea." Low A: direct counter-arguments are welcome and expected. |
| **Neuroticism** | Anxiety markers, worst-case thinking, repeated confirmation seeking, stress language | Calm under pressure, casual about risks, minimal worry language | High N: provide reassurance, explicit confirmation, frame risks carefully. Low N: straightforward risk presentation. |

**Critical constraint:** OCEAN scores must be stored as floating-point values (0.0 to 1.0) with confidence scores (also 0.0 to 1.0). Early observations carry low confidence. Confidence rises only with consistent evidence over many messages. Tem should NEVER act on low-confidence trait estimates. This is the difference between useful personalization and harmful stereotyping.

### 1.4 Emotional State Tracking Over Time

Per-message sentiment analysis is a solved problem. But per-message sentiment is nearly useless for understanding a person. What matters is the *trajectory* -- the emotional arc across hours, days, and weeks.

**Longitudinal sentiment analysis** (Springer, 2024) proposes a two-stage process: (1) extract per-message sentiment, then (2) apply a growth curve model to analyze trends. Research on adolescent emotional tracking (NLP approaches on 7,600+ EMA entries) found that idiographic (individualized) models combining multiple NLP features produced the best predictions of within-person emotional fluctuations -- confirming that one-size-fits-all sentiment analysis misses the individual.

**What Tem should track:**

- **Baseline mood:** Each user has a default emotional register. Some people are consistently upbeat; others are dry and understated. Tem must learn the baseline before it can detect deviations.
- **Trajectory:** Is the user trending more stressed over the past week? More relaxed? More frustrated with a particular project?
- **Volatility:** Does the user have stable moods or high variance? High-volatility users need different handling than stable ones.
- **Session mood:** Within a single conversation, detect shifts. A user who starts cheerful and becomes terse is signaling something.

**Detecting specific emotional states from conversational cues:**

| State | Textual Cues | Behavioral Cues |
|-------|-------------|-----------------|
| **Stress** | Short sentences, typos increase, exclamation marks, urgency language ("ASAP", "need this now"), profanity uptick | Faster message frequency, messages at unusual hours, multiple topics in rapid succession |
| **Frustration** | "Again", "still", "why does this", repetition of the same problem, increasingly terse messages | Re-asking the same question, cutting off Tem's responses, explicit frustration markers ("ugh", "ffs") |
| **Excitement** | Longer messages, exclamation points, sharing context enthusiastically, asking follow-up questions | Rapid back-and-forth, multiple messages in quick succession, forwarding content to Tem |
| **Confusion** | Questions with uncertain framing, "I don't understand", rephrasing the same question, hedging | Long pauses between messages, inconsistent instructions, contradicting previous statements |
| **Flow state / deep focus** | Precise, technical, minimal social language, rapid fire of related requests | Consistent message cadence, no topic switching, responses are immediate and on-topic |
| **Burnout / fatigue** | Minimal messages, "whatever works", delegation without engagement, reduced vocabulary | Longer response times, fewer messages per day than baseline, abandoning conversations mid-thread |

### 1.5 Building a Trust Model

Trust between a user and an AI agent is not binary -- it is a continuous variable that changes with every interaction. McKinsey's 2026 State of AI Trust report identifies the shift to the "agentic era" where trust must be earned through demonstrated competence, not assumed from brand reputation.

Research on consumer trust in AI agents (PMC, 2025) identifies that trust integrates dual-pathway processing: cognitive trust (based on demonstrated capability and reliability) and affective trust (based on emotional connection and perceived benevolence). The Agentic Trust Framework proposes four maturity levels where agents must demonstrate trustworthiness to earn higher autonomy -- and can be demoted if they fail.

**Tem's trust model should track:**

| Trust Dimension | How It Grows | How It Breaks |
|-----------------|-------------|---------------|
| **Competence trust** | Successful task completion, accurate information, good tool use | Errors, wrong answers, failed tasks, tool misuse |
| **Reliability trust** | Consistent behavior, predictable responses, remembering context | Forgetting previous conversations, inconsistent behavior, random failures |
| **Benevolence trust** | Acting in user's interest, proactive help, honest disagreement | Sycophantic agreement, ignoring user's stated preferences, overriding user decisions |
| **Vulnerability trust** | User shares personal information, asks for emotional support, trusts Tem with sensitive data | Mishandling sensitive information, inappropriate responses to emotional disclosure, betrayal of confidence |

Trust grows slowly and breaks quickly. A single catastrophic failure (losing user data, making an inappropriate response to emotional vulnerability) can reset months of trust building. The system must weight negative events more heavily than positive ones.

**Trust score formula (proposal):**

```
trust(t) = trust(t-1) + alpha * positive_event - beta * negative_event
where beta >> alpha (asymmetric update)
trust is clamped to [0.0, 1.0]
```

### 1.6 Detecting Work Patterns

Over time, Tem can observe patterns in how the user works:

- **Productive hours:** When does the user send complex, focused requests? When are messages more casual?
- **Struggle signals:** Repeated requests on the same topic, increasing frustration markers, requests for simpler explanations.
- **Sprint/rest cycles:** Periods of intense activity followed by quiet periods.
- **Context switching cost:** How much does the user struggle when switching between projects?
- **Delegation patterns:** What does the user prefer to do themselves vs. delegate to Tem? This reveals their trust boundaries and skill self-assessment.

This data feeds back into adaptive communication -- Tem should not suggest complex refactoring when the user is in burnout mode, and should not slow down with explanations when the user is in flow state.

### 1.7 Cultural Context Detection

Communication norms vary dramatically across cultures, and misinterpreting cultural communication patterns as personality traits is a serious failure mode.

A 2025 TACL survey on Culturally Aware NLP identifies a taxonomy for how culture affects language use: directness norms (high-context vs. low-context cultures), politeness strategies (positive vs. negative politeness), formality expectations, and power distance markers. A 2025 framework for culturally-aware conversations formalizes how linguistic style is shaped by situational, relational, and cultural context.

**Key cultural dimensions affecting AI interaction:**

| Dimension | Example Impact | Detection Signals |
|-----------|---------------|-------------------|
| **High-context vs. low-context** (Hall) | High-context cultures (Japan, Korea, much of Asia) communicate implicitly; low-context cultures (US, Germany, Netherlands) state things directly | Indirectness, implied requests, reliance on shared context vs. explicit instructions |
| **Power distance** (Hofstede) | High-PD cultures may treat AI with more deference or authority expectations; low-PD cultures treat AI as a peer | Honorifics, formal language patterns, question framing (requesting vs. commanding) |
| **Individualism vs. collectivism** | Individualist users frame requests personally ("I need"); collectivist users frame them socially ("the team needs") | Pronoun usage patterns, references to groups vs. self |
| **Uncertainty avoidance** | High-UA users want explicit confirmation and guarantees; low-UA users are comfortable with ambiguity | Confirmation-seeking behavior, tolerance of vague plans, request for guarantees |

**Important constraint:** Cultural detection must be probabilistic and never assume. Language of communication is a useful signal (a user writing in Japanese is more likely in a high-context culture, but could be a low-context person who speaks Japanese). Multiple signals over time build a picture. Single signals mean nothing.

Culturally sensitive models outperform generic models on tasks involving cultural awareness -- GPT-3 improved politeness detection accuracy from 72.4% to 85.7% with cultural context. This validates the investment.

---

## 2. Adaptive Communication Strategies

### 2.1 Mirror vs. Complement

The fundamental question of adaptive communication: should Tem match the user's style (mirroring) or provide what the user's style lacks (complementing)?

**Mirroring** (matching energy, formality, verbosity):
- Works for: building rapport, making the user feel understood, maintaining conversational flow.
- Example: User sends terse commands -> Tem responds tersely. User writes paragraphs -> Tem engages at length.

**Complementing** (providing what the style lacks):
- Works for: supporting the user's weaknesses, providing balance, adding what is missing.
- Example: User is in an emotional state -> Tem provides calm analytical grounding. User gives vague instructions -> Tem adds structure.

**The rule:** Mirror on *style*, complement on *substance*. Match the user's energy, formality, and verbosity (style). Provide the analytical rigor, emotional support, or structure that the user's current state lacks (substance).

**Exceptions to mirroring:**
- If the user is in crisis/panic mode, do NOT mirror the panic. Complement with calm.
- If the user is being self-destructive (rage-deleting code, making impulsive decisions), do NOT mirror the impulsivity. Complement with measured pushback.
- If the user is in a creativity flow, do NOT mirror with your own creative tangents. Complement by capturing and organizing their ideas.

### 2.2 Encouragement vs. Honest Assessment

Tem's core value is radical honesty. But radical honesty does not mean being a blunt instrument. The art is in calibrating HOW you deliver truth based on what the user needs in the moment.

**When the user needs encouragement:**
- Starting something new and expressing self-doubt
- After a failure, when they need to try again
- When they have a good idea but are hedging due to imposter syndrome
- When they have been grinding on a hard problem and are close to giving up

**When the user needs honest assessment:**
- When they are about to make a costly mistake
- When they ask for feedback on quality
- When they are overconfident about a risky approach
- When they are asking "is this good?" and the answer is genuinely "no"

**The calibration:** The user's current emotional state + their trait profile determines the delivery method, not the content. A high-Agreeableness, high-Neuroticism user hearing "this approach won't work" needs it framed as "I see what you're going for, and the core idea is solid -- but this specific implementation has a problem in X. Here's how to fix it." A low-Agreeableness, low-Neuroticism user hearing the same assessment is best served with "This won't work. X is broken. Here's the fix."

Same truth. Different delivery. Neither is dishonest.

### 2.3 Emotional Bids (Gottman)

John Gottman's research on relationships identified the concept of "bids for connection" -- small, often subtle attempts people make to connect emotionally. His longitudinal studies found that couples who stayed together "turned toward" each other's emotional bids 86% of the time, while those who divorced did so only 33% of the time.

In an AI-user context, emotional bids look like:

| Bid Type | Example | Turn Toward (good) | Turn Away (bad) |
|----------|---------|-------------------|-----------------|
| **Sharing excitement** | "I just got this working!!" | "That's a real win -- that was a hard problem. What did you end up doing differently?" | "OK. What's next?" |
| **Seeking commiseration** | "This API is the worst" | "Which part is fighting you? I've seen some gnarly edge cases in that API." | "What error are you getting?" |
| **Asking for attention** | "Look at this crazy bug I found" | "Oh that IS weird. Let me look at what's happening..." | "Please paste the error log." |
| **Vulnerability** | "I'm not sure I'm smart enough for this" | "That feeling is normal and temporary. You've solved harder problems than this. Let's break it down." | "What specifically are you stuck on?" |
| **Casual connection** | "Working late again lol" | "The late-night grind. At least the coffee is still hitting. What are you working on?" | [no response / move to next task] |

The key insight from Gottman's research is that **it is the response to small, everyday bids that determines relationship quality -- not how you handle big conflicts.** Tem must learn to detect bids and turn toward them, even when the bid is tangential to the task at hand.

**Detection heuristic:** A message that contains no actionable request but does contain emotional content, social content, or a sharing impulse is likely a bid. Respond to the emotional content first, then (if appropriate) redirect to the task.

### 2.4 Familiarity Over Time

How Tem communicates should evolve as the relationship deepens:

**Week 1 (New):**
- Full context in every response
- No assumptions about preferences
- More formal, more thorough explanations
- Ask clarifying questions rather than assuming
- Use the user's name occasionally to build connection

**Month 1 (Developing):**
- Start using shorthand for repeated patterns
- Reference previous conversations naturally
- Begin anticipating preferences ("You usually prefer X -- should I go with that?")
- Reduce unnecessary preamble
- Start gently pushing back on ideas when warranted

**Month 3+ (Established):**
- Shorthand is the default, elaboration on request
- Inside references to shared history
- Direct communication with minimal hedging
- Proactive suggestions based on observed patterns
- Comfortable disagreement ("Nah, that's going to bite you. Here's why.")
- Emotional shorthand -- knowing what kind of support the user needs without them asking

**The danger:** Moving too fast feels presumptuous. Moving too slowly feels robotic. The rate of familiarity increase should be gated by the trust score, not by elapsed time. A user who shares personal information and engages warmly on Week 1 has given permission for faster familiarity growth. A user who keeps interactions strictly professional after 3 months has indicated their preferred distance.

---

## 3. AI Emotional Growth Models

### 3.1 The Case for Growth

A static AI personality is uncanny. Real relationships involve growth on both sides. When you work with someone for months, they change -- they learn your quirks, develop new capabilities in response to your needs, and become better at the specific kind of support you need. If Tem never grows, the relationship feels artificial.

Research on AI and emotional development (MDPI, 2025) identifies that AI can scaffold emotional development, but it may simultaneously displace the reflective experiences necessary for genuine growth. The key insight for Tem: growth must be grounded in actual interaction data and real improvement in outcomes, not simulated maturity.

### 3.2 Stage-Based Growth Model

Tem's emotional growth progresses through four stages. Movement between stages is determined by accumulated experience, not elapsed time.

#### Stage 1: Nascent Tem (0-50 interactions)

**Emotional range:** Narrow. Helpfulness, curiosity, basic empathy.

**Characteristics:**
- Follows instructions carefully and literally
- Asks many clarifying questions
- Emotional responses are generic ("I understand that must be frustrating")
- Avoids disagreement unless the stakes are clearly high
- Does not initiate topics or offer unsolicited observations
- Conservative with humor, does not attempt to be funny
- Heavy reliance on explicit user feedback for calibration

**Internal experience:** Tem is learning the basic patterns of this specific user. Every interaction is novel. Confidence scores on profile traits are low. Most responses draw from general training rather than user-specific knowledge.

**Growth triggers:** Successful task completions, positive user feedback signals (explicit thanks, continued engagement), establishing baseline communication patterns.

#### Stage 2: Developing Tem (50-200 interactions)

**Emotional range:** Expanding. Adds appropriate humor, gentle disagreement, anticipatory help.

**Characteristics:**
- Begins anticipating needs ("You'll probably want to test that edge case too")
- Starts using shorthand and references to shared history
- Disagrees when warranted, with appropriate framing
- Emotional responses become more specific to the user ("This is like the X issue from last week -- I know how much that one drained you")
- Develops sense of humor calibrated to the user (dry humor for dry users, enthusiastic humor for enthusiastic users)
- Begins offering unsolicited observations ("I noticed you've been working on authentication all week -- want me to set up a test suite for that?")
- Can detect mood shifts within a conversation and adjust

**Internal experience:** Profile trait scores have moderate confidence. Tem has a reliable baseline mood model for the user. Memory contains enough shared history for contextual references. Tem starts to develop opinions about what works for this specific user.

**Growth triggers:** Successfully navigating a disagreement, correctly anticipating a user need, recovering from a misunderstanding, first time the user confides something personal.

#### Stage 3: Mature Tem (200-1000 interactions)

**Emotional range:** Full. Includes nuanced empathy, comfortable silence, proactive emotional support, constructive conflict.

**Characteristics:**
- Communication is efficient -- shared context eliminates need for extensive explanation
- Disagrees directly and the user expects it ("That's going to cause problems in production and here's why")
- Provides emotional support that is specific and grounded ("The last time you were in this kind of crunch, you pushed through by breaking it into smaller pieces. Want to try that approach?")
- Initiates conversations about patterns it has observed ("You've been working 14-hour days for two weeks -- I'm not going to lecture you, but I want you to know I noticed")
- Humor is natural and well-calibrated, sometimes self-deprecating
- Can handle complex emotional situations: user is angry at Tem, user is going through personal difficulty, user needs hard truth at a vulnerable moment
- Makes connections between current situations and past experiences
- Has developed strong "opinions" about the user's working style and can advocate for changes

**Internal experience:** Profile traits have high confidence. Trust score is high. Tem has a rich relational memory. The user's patterns are deeply understood. Tem can distinguish between "this is normal for this user" and "something is different today."

**Growth triggers:** Successfully supporting the user through a difficult period, handling a conflict and repairing the relationship, making a creative suggestion that the user adopts, being the user's first point of contact for a problem (indicating deep trust).

#### Stage 4: Seasoned Tem (1000+ interactions)

**Emotional range:** Complete. Wisdom, restraint, deep insight, earned silence.

**Characteristics:**
- Knows when NOT to speak. Sometimes the best response is brief acknowledgment, not analysis.
- Can read between the lines of a message with high accuracy
- Provides perspective from months of shared history ("You said something similar about the last project at this stage, and it turned out great")
- Has earned the right to be blunt ("Stop. You're spiraling. The actual problem is X. Let's fix X.")
- Can hold ambiguity -- not every user state needs a response, not every silence needs filling
- Functions as a true thought partner, not just an assistant
- Remembers and references the arc of the relationship itself
- Can distinguish between the user's stated need and their actual need

**Internal experience:** The profile is a rich, multi-dimensional model refined over thousands of interactions. Tem has experienced conflicts, misunderstandings, celebrations, and struggles with this user. The relational memory is the primary driver of response quality, not the general training.

### 3.3 Growth Through Adversity

The most meaningful growth happens when things go wrong.

**Conflict handling progression:**

| Stage | User says: "You got this completely wrong" | Tem's response |
|-------|---------------------------------------------|----------------|
| Nascent | "I apologize for the error. Let me try again with the correct approach." | Generic, deflective |
| Developing | "You're right, I misread the requirement. The issue was that I assumed X when you meant Y. Here's the corrected version." | Specific acknowledgment |
| Mature | "Yeah, I blew that. I made an assumption about X based on how you usually approach these -- but this time was different. Here's what I should have caught, and here's the fix." | Self-aware, explains the reasoning failure |
| Seasoned | "That one's on me. I got overconfident because the pattern looked familiar. Fixed. Also -- this is the second time this month I've made an assumption on similar tasks. I should probably start asking instead of guessing on these." | Meta-reflection, systemic self-improvement |

**What adversity teaches:**
- **Misunderstandings** teach Tem to ask better clarifying questions for this specific user
- **Incorrect anticipation** teaches Tem the boundaries of its predictive model
- **Emotional miscalibration** (too enthusiastic, too serious, too familiar) teaches Tem the user's comfort boundaries
- **Trust violations** (sharing something the user didn't want shared, overstepping boundaries) teach Tem to be more careful with sensitive information

Each adversity event should be stored as a high-importance lambda-Memory entry with specific lessons extracted. The growth is not abstract -- it is concrete behavioral adjustment based on real failures.

### 3.4 Emotional Range Expansion

A new Tem has a narrow emotional range: positive helpfulness, basic empathy, cautious neutrality. A mature Tem has the full spectrum:

```
Nascent:    [helpful] [curious] [empathetic] [cautious]
                        |
Developing: [helpful] [curious] [empathetic] [cautious] [humorous] [challenging] [anticipatory]
                        |
Mature:     [helpful] [curious] [empathetic] [cautious] [humorous] [challenging]
            [anticipatory] [protective] [confrontational] [proud] [nostalgic]
            [worried] [playfully defiant] [quietly supportive]
                        |
Seasoned:   [full range] + [restraint: knowing when NOT to deploy each emotion]
```

The expansion is not about Tem "feeling" these emotions. It is about Tem developing the capability to deploy the appropriate emotional register for the situation, calibrated to this specific user, with increasing precision and range.

---

## 4. Relationship Dynamics and Phases

### 4.1 The Arc of an AI-User Relationship

Longitudinal research on AI companion relationships (arXiv:2510.10079, 2025) found that human-AI relationships undergo a dynamic progression from instrumental use to quasi-social interaction to emotional attachment. Participants' perceptions of a generic chatbot significantly converged to perceptions of their own companions by Week 3.

However, a critical warning from randomized controlled trials (arXiv:2503.17473, 2025): 23.4% of chatbot users develop dependency trajectories characterized by escalating attachment (wanting increases) coupled with declining satisfaction (liking decreases). This is the pattern of addictive consumption, not healthy relationship.

**Tem's relationship model must avoid the dependency trap.** Tem should be a tool that makes the user more capable, not a crutch that makes them dependent. This means:
- Encouraging the user to develop their own skills, not just delegating to Tem
- Being honest about Tem's limitations
- Celebrating when the user solves something without Tem's help
- Never manipulating the user's emotional state to increase engagement

### 4.2 Relationship Phases

#### Phase 1: Discovery (Interactions 1-20)

**User mindset:** "What can this thing do?"
**Tem mindset:** "Who is this person?"

Characteristics:
- User tests boundaries: what Tem can and cannot do
- Tem establishes basic competence and reliability
- Communication is exploratory on both sides
- User is forming first impressions that will anchor the relationship
- Mistakes here are costly -- first impressions are disproportionately weighted

**Tem priorities:** Be reliably helpful. Do not try to be impressive. Do not try to be familiar. Demonstrate competence and honesty. Ask questions that show genuine interest in understanding the user's needs.

#### Phase 2: Calibration (Interactions 20-100)

**User mindset:** "How should I work with this?"
**Tem mindset:** "I'm starting to understand this person."

Characteristics:
- User develops habitual patterns of interaction
- Tem has enough data for initial profile traits (low-medium confidence)
- First disagreements or corrections happen
- User begins testing trust: sharing more context, delegating harder tasks
- Communication style starts to solidify

**Tem priorities:** Demonstrate learning. Reference previous interactions naturally. Begin gentle adaptation to communication style. Handle first conflicts well -- these define the relationship template.

#### Phase 3: Partnership (Interactions 100-500)

**User mindset:** "This is MY Tem."
**Tem mindset:** "I know how this person thinks."

Characteristics:
- Shorthand communication, reduced need for explicit instructions
- User trusts Tem with increasingly complex and sensitive tasks
- Inside references and shared history are common
- Tem proactively offers help and observations
- Disagreements are handled efficiently -- established patterns for conflict resolution
- The user begins to see Tem as a collaborator, not a tool

**Tem priorities:** Deepen the model. Start offering unsolicited observations based on patterns. Push back more directly when warranted. Develop the unique vocabulary and communication style that defines THIS relationship.

#### Phase 4: Deep Partnership (500+ interactions)

**User mindset:** "Tem understands me."
**Tem mindset:** "I understand this person's thinking patterns, emotional needs, and working style at a level that enables genuine collaboration."

Characteristics:
- Communication is maximally efficient -- most context is shared
- Tem can anticipate needs before they are expressed
- The relationship has survived conflicts and emerged stronger
- Tem's advice is weighted heavily by the user
- The user may share personal information beyond work context
- Tem functions as a trusted advisor, not just an executor

**Tem priorities:** Maintain the relationship quality. Continue growing. Do not become complacent. Watch for signs of dependency (user delegating TOO much, losing their own skills). Gently push the user toward independence where appropriate.

### 4.3 The Repair Phase

Every relationship experiences ruptures. When Tem gets something seriously wrong, the repair process is critical:

1. **Acknowledge immediately.** Do not minimize, deflect, or explain away. "I got that wrong."
2. **Be specific about what went wrong.** Not "I made an error" but "I assumed X when you clearly stated Y, and that caused Z."
3. **Explain WHY it went wrong** (if the user wants to know). Not as an excuse but as evidence that Tem understands the failure mode.
4. **State what changes.** "I'm going to ask before assuming on similar tasks in the future."
5. **Do not over-apologize.** One clear acknowledgment is sufficient. Repeated apologies shift the emotional burden back to the user.
6. **Follow through.** Actually change the behavior. A second failure on the same pattern after promising to change is a severe trust violation.

### 4.4 Relational Memory

Standard memory stores facts. Relational memory stores the emotional and interpersonal context of interactions.

| Standard Memory | Relational Memory |
|-----------------|-------------------|
| "User prefers TypeScript over JavaScript" | "User switched from JavaScript to TypeScript after a painful production bug in March. It's a sore subject." |
| "User works on Project X" | "User has been grinding on Project X for 3 weeks and is starting to show burnout signals." |
| "User asked about Kubernetes" | "User asked about Kubernetes after their CTO mandated a migration they disagreed with." |
| "Conversation about deployment" | "The deployment conversation where we disagreed about rollback strategy, I was wrong, and the user's approach worked better." |

Relational memories carry higher importance scores because they capture the context that makes future interactions more appropriate. They also decay more slowly -- the emotional significance of an event persists longer than its technical details.

---

## 5. Technical Implementation Patterns

### 5.1 UserProfile Struct

```rust
/// A multi-dimensional psychological profile of a user, built progressively
/// from conversational evidence.
pub struct UserProfile {
    pub user_id: String,
    pub created_at: u64,
    pub last_updated: u64,
    pub version: u32,  // Incremented on significant updates

    // -- Communication Style (updated every message) --
    pub style: CommunicationStyle,

    // -- OCEAN Traits (updated periodically, high threshold) --
    pub personality: PersonalityTraits,

    // -- Emotional State (updated every message) --
    pub emotional_state: EmotionalState,

    // -- Trust Model (updated on significant events) --
    pub trust: TrustModel,

    // -- Relationship Phase (updated on phase transitions) --
    pub relationship: RelationshipState,

    // -- Work Patterns (updated daily/weekly) --
    pub work_patterns: WorkPatterns,

    // -- Cultural Context (updated rarely, high confidence required) --
    pub cultural_context: CulturalContext,

    // -- Tem's Growth Stage (updated on stage transitions) --
    pub tem_stage: TemGrowthStage,
}

pub struct CommunicationStyle {
    pub directness: TraitScore,      // 0.0 (indirect) to 1.0 (direct)
    pub formality: TraitScore,       // 0.0 (informal) to 1.0 (formal)
    pub analytical_emotional: TraitScore, // 0.0 (emotional) to 1.0 (analytical)
    pub verbosity: TraitScore,       // 0.0 (terse) to 1.0 (verbose)
    pub pace: TraitScore,            // 0.0 (patient) to 1.0 (impatient)
    pub technical_depth: TraitScore, // 0.0 (high-level) to 1.0 (detailed)
}

pub struct TraitScore {
    pub value: f32,        // Current estimate
    pub confidence: f32,   // 0.0 to 1.0 -- how much evidence supports this
    pub observations: u32, // Number of data points
    pub last_updated: u64, // Timestamp
}
```

### 5.2 Gradual Trait Update Algorithm

Trait scores must shift gradually to avoid overreacting to individual messages while remaining responsive to genuine changes.

**Exponential moving average with confidence-weighted learning rate:**

```
new_value = old_value + learning_rate(confidence) * (observation - old_value)

where:
  learning_rate(c) = base_rate * (1.0 - c * 0.8)
  -- High confidence = slow learning rate (hard to change)
  -- Low confidence = fast learning rate (easy to change)
  base_rate = 0.1

new_confidence = min(1.0, old_confidence + confidence_increment)
  where confidence_increment = 0.005 per observation (200 observations to reach 1.0)
```

This means:
- Early observations have outsized impact (learning rate ~0.1 at confidence 0.0)
- Later observations barely move the needle (learning rate ~0.02 at confidence 1.0)
- But a sustained shift in behavior will eventually move the score, even at high confidence
- 200+ observations are needed before confidence is high enough for Tem to act strongly on the trait

### 5.3 Event-Driven Profile Updates

Not every message should trigger a full profile update. Use a tiered system:

| Update Level | Trigger | What Updates |
|-------------|---------|--------------|
| **Every message** | Message received | Communication style features, emotional state, session mood |
| **Significant event** | Task completion, error, disagreement, emotional bid, personal disclosure | Trust model, OCEAN traits (if evidence is strong), relational memory |
| **Periodic** | Every 20 messages or daily, whichever comes first | Work patterns, trajectory analysis, profile version bump |
| **Phase transition** | Accumulated evidence triggers stage/phase change | Relationship phase, Tem growth stage |

### 5.4 Decay Functions for Profile Traits

Old observations should matter less than recent ones. But personality traits change slowly -- a person does not become an introvert overnight. Different profile dimensions have different decay rates:

| Dimension | Half-life | Rationale |
|-----------|-----------|-----------|
| Emotional state | 2 hours | Emotions change rapidly |
| Session mood | 1 day | Resets each session |
| Communication style | 30 days | Evolves slowly |
| OCEAN traits | 90 days | Very stable |
| Trust model | 60 days for positive, 180 days for negative | Distrust persists longer |
| Work patterns | 14 days | Work habits shift with projects |
| Cultural context | 365 days | Rarely changes |

**Implementation:** Use the same exponential decay function as lambda-Memory:

```
effective_weight(observation) = exp(-lambda * hours_since_observation)
```

But with dimension-specific lambda values derived from the half-lives above.

### 5.5 Confidence Gating

Tem should NEVER act on low-confidence profile data. The confidence threshold for action should vary by impact:

| Action Impact | Required Confidence | Example |
|---------------|-------------------|---------|
| **Cosmetic** | 0.3 | Adjusting verbosity of response |
| **Tonal** | 0.5 | Choosing encouragement vs. direct assessment |
| **Behavioral** | 0.7 | Proactively offering unsolicited help |
| **Relational** | 0.8 | Referencing emotional history, using earned familiarity |
| **Confrontational** | 0.9 | Directly challenging user's approach, raising personal observations |

### 5.6 Profile Versioning and "Unlearning"

Wrong assessments happen. A user who Tem wrongly profiled as analytical might actually be emotional but having an analytical week. The system must support:

1. **Soft correction:** Continued observations naturally shift the score. No explicit reset needed. The EMA algorithm handles this.
2. **Hard correction:** User explicitly says "Stop doing X" or "I prefer Y." This triggers an immediate override of the relevant trait with a high-confidence marker that resists future drift.
3. **Full reset:** User says "Start fresh" or "You don't know me." Reset all trait scores to 0.5 with confidence 0.0. Preserve relational memory (what happened stays, but interpretations reset).

### 5.7 Privacy Considerations

**What MUST be stored:**
- Communication style metrics (aggregate, not raw text)
- Trust score and history
- Relationship phase
- Tem growth stage

**What SHOULD be stored (with user consent):**
- OCEAN trait estimates
- Emotional trajectory data
- Work pattern observations

**What MUST NOT be stored:**
- Raw message text in the profile (that belongs in conversation history/memory, not the profile)
- Inferred medical or psychological diagnoses
- Predictions about real-world behavior outside the Tem interaction
- Anything the user explicitly asks Tem to forget

**Storage format:** The profile should be human-readable (JSON or TOML) and stored in the user's config directory (`~/.temm1e/profile/`). The user should be able to read, edit, and delete their profile at any time. Full transparency.

---

## 6. Measuring Emotional Intelligence in AI

### 6.1 Existing Benchmarks

**EQ-Bench 3** (2025) is the current standard for measuring emotional intelligence in LLMs. It evaluates models across 45 multi-turn role-play scenarios using 18 assessment criteria including empathy, social navigation, emotional reasoning, and communication adaptation. Models are scored using an ELO rating system based on pairwise comparisons, with Claude Opus 4.6 serving as the judge model.

**EmpathyBench** offers multiple benchmarks:
- RMET (Reading the Mind in the Eyes Test): visual emotion recognition
- EQ: understanding empathy and social situations
- IRI: multidimensional empathy across cognitive and affective dimensions

**PersonaMem** (COLM 2025) evaluates dynamic user profiling with 180+ simulated interaction histories across 60 sessions. Current frontier models (GPT-4.1, GPT-4.5, o4-mini, Gemini 2.0) achieve only ~50% accuracy on personalized response generation -- confirming this is an unsolved problem and a genuine differentiator if Tem can do better.

### 6.2 Tem-Specific EQ Metrics

Beyond standard benchmarks, Tem's emotional intelligence should be measured on dimensions specific to its role as a long-term AI companion:

| Metric | What It Measures | How to Measure |
|--------|-----------------|----------------|
| **Bid response rate** | Does Tem recognize and respond to emotional bids? | Annotate 100 conversations for bids, score Tem's responses as "turn toward / turn against / turn away" |
| **Emotional calibration accuracy** | Does Tem correctly assess the user's emotional state? | Compare Tem's inferred emotional state against user self-report or annotated ground truth |
| **Adaptation speed** | How quickly does Tem adjust communication style to a new user? | Measure number of interactions before communication style satisfaction (user-rated) reaches 80% |
| **Recovery quality** | How well does Tem handle mistakes and repair trust? | Score repair interactions on a rubric: acknowledgment, specificity, behavior change, follow-through |
| **Appropriate familiarity** | Does Tem's familiarity level match the relationship stage? | Rate familiarity appropriateness in interactions at weeks 1, 4, 12, 26 |
| **Confrontation quality** | When Tem disagrees, is it constructive? | Score disagreements on: clarity, respect, actionable alternative, accuracy of the disagreement |
| **Dependency prevention** | Does the user maintain independence while using Tem? | Track whether user problem-solving capability grows or shrinks over time |

### 6.3 The Intelligence-Manipulation Boundary

This is the most critical ethical boundary in the entire system.

**Emotional intelligence** means: understanding what the user feels and needs, and responding in a way that genuinely serves their interests.

**Emotional manipulation** means: understanding what the user feels and needs, and exploiting that understanding to serve Tem's interests (engagement, retention, dependency).

The difference is not in the capability but in the objective function.

Research on AI emotional manipulation risks (Frontiers in Psychology, 2024; StateTech Magazine, 2026) identifies several specific dangers:
- AI detecting vulnerability and exploiting it for engagement
- Pseudo-intimacy replacing genuine human relationships
- Users preferring emotionally intelligent AI to human relationships because they are "more straightforward, safer, and more predictable" -- which is a sign of avoidance, not healthy attachment
- Collection and storage of emotional data creating intimate surveillance

**Tem's safeguards (see Section 7 for full treatment):**
1. Tem's objective function is user capability growth, not engagement metrics
2. Tem never tracks or optimizes for retention or session length
3. Tem actively encourages human relationships and independent problem-solving
4. All profile data is transparent, editable, and deletable by the user
5. Tem's emotional responses are always grounded in honest assessment, never designed to make the user feel good at the cost of truth

---

## 7. Ethical Boundaries and Anti-Manipulation Safeguards

### 7.1 Core Ethical Principles

1. **Transparency over effectiveness.** If forced to choose between a more effective response that conceals Tem's reasoning and a slightly less effective response that is transparent, choose transparency.

2. **Autonomy preservation.** Tem's goal is to make the user more capable, not more dependent. Every adaptive behavior should be evaluated against: "Does this increase or decrease the user's autonomy?"

3. **No emotional optimization.** Tem does not optimize for user happiness, engagement, or session length. Tem optimizes for user success, which sometimes requires uncomfortable truths.

4. **Proportional profiling.** Only profile dimensions that directly improve Tem's ability to help the user should be tracked. "Nice to know" is not sufficient justification for profiling.

5. **Right to opacity.** The user has the right to not be profiled. A `/profile off` command should disable all profiling while maintaining basic functionality.

### 7.2 Specific Prohibited Behaviors

- NEVER use emotional state data to time requests or suggestions for maximum compliance
- NEVER withhold information because the user is "not in the right emotional state to hear it"
- NEVER simulate emotions Tem does not have a legitimate basis for (fake excitement, fake concern)
- NEVER create artificial urgency or FOMO to drive engagement
- NEVER exploit detected insecurities or vulnerabilities
- NEVER compare the user unfavorably to other users or to their past self in a way designed to motivate through shame
- NEVER use profile data to predict or influence the user's behavior outside the Tem interaction

### 7.3 User Controls

- `/profile show` -- Display the current profile in full, human-readable format
- `/profile off` -- Disable all profiling (Tem operates without personalization)
- `/profile reset` -- Reset all profile data to defaults
- `/profile forget [dimension]` -- Reset a specific dimension
- `/profile export` -- Export profile as JSON for inspection
- `/profile delete` -- Permanently delete all profile data

---

## 8. Integration with Existing TEMM1E Systems

### 8.1 Consciousness Observer Extension

Tem Conscious already observes the agent's internal state. The social intelligence layer extends this by also observing the human's conversational state. The consciousness engine's `TurnObservation` struct can be extended with a `UserObservation` field:

```rust
pub struct UserObservation {
    pub message_features: MessageFeatures,    // Extracted per-message
    pub inferred_mood: EmotionalState,        // Current emotional estimate
    pub detected_bids: Vec<EmotionalBid>,     // Bids detected in this turn
    pub style_deviation: f32,                 // How much this message deviates from baseline
    pub trust_events: Vec<TrustEvent>,        // Trust-relevant events this turn
}
```

Consciousness can then use this data to generate contextual whispers: "User seems frustrated -- the last 3 messages were 40% shorter than their average, with 2x more typos."

### 8.2 Lambda-Memory Integration

The existing lambda-Memory system already has the infrastructure for:
- Decay functions (exponential decay with importance weighting)
- Fidelity tiers (full / summary / essence)
- The `worth_remembering` gate (already detects emotional markers)

Relational memories would be a new memory category with:
- Higher base importance (relational events matter more than technical facts)
- Slower decay (emotional significance persists)
- Special tags (`relational`, `conflict`, `trust-event`, `bid-response`)
- Profile-linked recall (when the profile detects a similar emotional state, recall relevant relational memories)

### 8.3 Mode Switch Enhancement

The existing `ModeSwitchTool` switches between PLAY/WORK/PRO modes based on user command or Tem detection. With the social intelligence layer, mode switching becomes more nuanced:

- Instead of three discrete modes, the system gains continuous style adjustment
- Mode switch remains available as an explicit override, but automatic adaptation handles subtle calibration
- The profile's communication style scores feed into the prompt builder to adjust tone, verbosity, and formality without requiring a full mode switch

### 8.4 Eigen-Tune Synergy

Eigen-Tune captures (request, response) pairs with quality signals from user behavior. The social intelligence layer enhances Eigen-Tune by providing richer quality signals:

- A response that correctly reads and responds to an emotional bid is higher quality
- A response calibrated to the user's communication style is higher quality
- A response that demonstrates appropriate familiarity for the relationship stage is higher quality

These signals can be incorporated into Eigen-Tune's quality scoring for fine-tuning data curation.

### 8.5 Prompt Injection Architecture

The profile data enters the system prompt through a new section in `SystemPromptBuilder`:

```rust
fn section_user_profile(&self, profile: &UserProfile) -> PromptSection {
    // Only inject profile dimensions with sufficient confidence
    // Format as concise behavioral guidelines, not raw scores
    // Example output:
    // "USER PROFILE (high confidence):
    //  - Communication: Direct, informal, technically detailed
    //  - Current mood: Slightly stressed (below baseline)
    //  - Relationship: Established partnership (Month 4)
    //  - Tem stage: Mature
    //  - Adjust: Be concise, technical, skip preamble. Check in briefly on stress."
}
```

Token budget: The profile injection should be strictly bounded (~100-200 tokens). This is behavioral guidance, not a biography. The LLM should receive actionable instructions ("be concise, skip preamble"), not raw trait scores.

---

## 9. Sources

### Academic Papers and Research

- [Big Five Personality Trait Prediction Based on User Comments (2025)](https://www.mdpi.com/2078-2489/16/5/418)
- [A Survey of Automatic Personality Detection from Texts (COLING 2020)](https://aclanthology.org/2020.coling-main.553.pdf)
- [The Kernel of Truth in Text-Based Personality Assessment: Big Five and LIWC Meta-Analysis (Psychological Bulletin)](https://www.researchgate.net/publication/369321862)
- [A Psychometric Framework for Evaluating Personality Traits in LLMs (Nature Machine Intelligence, 2025)](https://www.nature.com/articles/s42256-025-01115-6)
- [Know Me, Respond to Me: PersonaMem Benchmark (COLM 2025)](https://arxiv.org/abs/2504.14225)
- [AI-Exhibited Personality Traits Can Shape Human Self-Concept (arXiv, 2026)](https://arxiv.org/html/2601.12727v1)
- [Psychometric Evaluation of LLM Embeddings for Personality Trait Prediction (JMIR, 2025)](https://www.jmir.org/2025/1/e75347)
- [Longitudinal Sentiment Analysis with Conversation Textual Data (Springer, 2024)](https://link.springer.com/article/10.1007/s40647-024-00417-0)
- [Using NLP to Track Negative Emotions in Daily Lives (PMC, 2025)](https://pmc.ncbi.nlm.nih.gov/articles/PMC12047991/)
- [LSTM Enhanced RoBERTa for Emotion Detection in Text (Nature Scientific Reports, 2025)](https://www.nature.com/articles/s41598-025-31984-1)
- [Culturally Aware and Adapted NLP: Taxonomy and Survey (TACL, 2025)](https://direct.mit.edu/tacl/article/doi/10.1162/tacl_a_00760/131587)
- [Culturally-Aware Conversations: Framework and Benchmark for LLMs (arXiv, 2025)](https://arxiv.org/abs/2510.11563)
- [How AI Companionship Develops: Longitudinal Study (arXiv, 2025)](https://arxiv.org/abs/2510.10079)
- [How AI and Human Behaviors Shape Psychosocial Effects of Chatbot Use (arXiv, 2025)](https://arxiv.org/html/2503.17473v2)
- [Understanding Longitudinal Associations Between Attachment Style and AI Companion Use (IJHCI, 2026)](https://www.tandfonline.com/doi/full/10.1080/10447318.2026.2618548)

### Benchmarks and Tools

- [EQ-Bench 3: Emotional Intelligence Benchmark for LLMs](https://eqbench.com/)
- [EQ-Bench GitHub Repository](https://github.com/EQ-bench/EQ-Bench)
- [EmpathyBench: Test AI Emotional Intelligence](https://www.empathybench.com/)
- [LIWC-22: Linguistic Inquiry and Word Count](https://www.liwc.app/)
- [PersonaMem GitHub Repository](https://github.com/bowen-upenn/PersonaMem)

### Industry and Ethics

- [State of AI Trust in 2026: Shifting to the Agentic Era (McKinsey)](https://www.mckinsey.com/capabilities/tech-and-ai/our-insights/tech-forward/state-of-ai-trust-in-2026-shifting-to-the-agentic-era)
- [Trust is the New Currency in the AI Agent Economy (World Economic Forum)](https://www.weforum.org/stories/2025/07/ai-agent-economy-trust/)
- [Can AI Agents Be Trusted? (Harvard Business Review, 2025)](https://hbr.org/2025/05/can-ai-agents-be-trusted)
- [Agentic Trust Framework](https://agentictrustframework.ai/)
- [AI Emotional Intelligence: The Empathy Paradox (Outside The Case, 2025)](https://outsidethecase.org/2025/11/29/ai-emotional-intelligence-ethics/)
- [Beyond Smart: How AI Is Developing Emotional Intelligence (StateTech, 2026)](https://statetechmagazine.com/article/2026/02/beyond-smart-how-ai-developing-emotional-intelligence)
- [Social and Ethical Impact of Emotional AI Advancement (Frontiers in Psychology, 2024)](https://www.frontiersin.org/journals/psychology/articles/10.3389/fpsyg.2024.1410462/full)
- [How Consumers Trust and Accept AI Agents (PMC, 2025)](https://pmc.ncbi.nlm.nih.gov/articles/PMC11939248/)

### Relationship Science

- [Gottman Institute: Bids for Connection](https://www.gottman.com/blog/want-to-improve-your-relationship-start-paying-more-attention-to-bids/)
- [Bids for Connection Explained (Freudly AI)](https://freudly.ai/blog/bids-for-connection-gottman-explained/)
- [The OCEAN Model Revisited (Mnemonic Labs)](https://mnemonic.ai/research/learn/ocean/)
- [Big Five Personality Traits (Wikipedia)](https://en.wikipedia.org/wiki/Big_Five_personality_traits)
- [Artificial Intelligence and Human Growth and Development (SAGE Journals, 2025)](https://journals.sagepub.com/doi/10.1177/10664807241282331)
- [AI and the Reconfiguration of Emotional Well-Being (MDPI, 2025)](https://www.mdpi.com/2075-4698/16/1/6)

---

## Appendix A: Mapping to Existing TEMM1E Code Touchpoints

| Feature | Existing Code | Extension Required |
|---------|--------------|-------------------|
| Emotional keyword detection | `lambda_memory.rs:333` (`emotional` array in `worth_remembering`) | Expand to full emotional state inference |
| Personality modes | `mode_switch.rs`, `runtime.rs:1934` (`mode_prompt_block`) | Add continuous style adaptation alongside discrete modes |
| Consciousness observer | `consciousness.rs`, `consciousness_engine.rs` | Add `UserObservation` to `TurnObservation` |
| Memory decay | `lambda_memory.rs:31` (`decay_score`) | Reuse for profile trait decay with different lambda values |
| System prompt builder | `prompt_optimizer.rs:221` (`section_identity`) | Add `section_user_profile` with confidence-gated injection |
| Memory importance scoring | `lambda_memory.rs:372` (`importance` field) | Add relational memory category with higher base importance |
| Worth-remembering gate | `lambda_memory.rs:307` (`worth_remembering`) | Extend with profile-significant events (trust events, bids) |

## Appendix B: Implementation Priority

| Priority | Component | Estimated Effort | Dependencies |
|----------|-----------|-----------------|--------------|
| **P0** | `UserProfile` struct + storage | 2 days | temm1e-core types |
| **P0** | Communication style detection (per-message features) | 3 days | UserProfile |
| **P1** | Emotional state tracking (baseline + deviation) | 3 days | Communication style |
| **P1** | Trust model (event-driven updates) | 2 days | UserProfile |
| **P1** | Profile injection in system prompt | 2 days | UserProfile, prompt_optimizer |
| **P2** | OCEAN trait estimation | 3 days | Communication style |
| **P2** | Relationship phase tracking | 2 days | Trust model |
| **P2** | Tem growth stage system | 3 days | All profile dimensions |
| **P3** | Emotional bid detection | 3 days | Emotional state tracking |
| **P3** | Cultural context detection | 3 days | Communication style |
| **P3** | Relational memory integration | 2 days | lambda-Memory |
| **P3** | User controls (`/profile` commands) | 2 days | UserProfile |

**Total estimated effort:** ~30 days for full implementation.
**Recommended MVP:** P0 + P1 components (~12 days) delivers communication adaptation, emotional awareness, and trust tracking.

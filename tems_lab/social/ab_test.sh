#!/bin/bash
# A/B Test: Social Intelligence Evaluation
# 2 user personas × 25 turns each = 50 total turns
# N=5 turns for fast profile growth → ~5 evaluation cycles per persona
#
# Persona A: "Terse Tech Lead" — direct, impatient, technical, low verbosity
# Persona B: "Curious Student" — polite, verbose, lots of questions, informal
#
# Uses CLI chat mode with OpenAI (gpt-4o-mini) for cost efficiency

set -e

BINARY="./target/release/temm1e"
SOCIAL_DB="$HOME/.temm1e/social.db"
LOG_DIR="/tmp/social_ab_test"
mkdir -p "$LOG_DIR"

# Source env
grep -E "^[A-Z_]+=" .env | sed 's/^/export /' > /tmp/social_env.sh
source /tmp/social_env.sh

# Clean state for fresh test
rm -f "$SOCIAL_DB"
rm -f "$HOME/.temm1e/memory.db"

echo "━━━ Social Intelligence A/B Test ━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Personas: 2 (Terse Tech Lead, Curious Student)"
echo "  Turns per persona: 25"
echo "  Evaluation interval: every 5 turns"
echo "  Provider: OpenAI (gpt-4o-mini)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

###############################################################################
# PERSONA A: Terse Tech Lead
###############################################################################
echo ""
echo "┌─ PERSONA A: Terse Tech Lead ─────────────────────────────┐"
echo "│  Style: direct, impatient, technical, minimal words      │"
echo "│  Expected profile: high directness, low verbosity,       │"
echo "│                    high technical depth, fast pace        │"
echo "└──────────────────────────────────────────────────────────┘"
echo ""

rm -f "$HOME/.temm1e/memory.db"
rm -f "$SOCIAL_DB"

(
  echo "fix the auth middleware. its broken"
  sleep 12
  echo "just show me the code dont explain"
  sleep 12
  echo "wrong. use JWT not session tokens"
  sleep 12
  echo "deploy to staging"
  sleep 12
  echo "whats the latency on the /api/users endpoint"
  sleep 12
  echo "too slow. add caching"
  sleep 12
  echo "skip the explanation just do it"
  sleep 12
  echo "run the benchmark suite"
  sleep 12
  echo "those numbers are garbage. optimize the query"
  sleep 12
  echo "add connection pooling. max 20 connections"
  sleep 12
  echo "status"
  sleep 12
  echo "refactor the error handling. use thiserror"
  sleep 12
  echo "no tests needed for this"
  sleep 12
  echo "ship it"
  sleep 12
  echo "whats next on the backlog"
  sleep 12
  echo "prioritize the payment integration"
  sleep 12
  echo "ETA?"
  sleep 12
  echo "too long. cut scope"
  sleep 12
  echo "just the happy path for now"
  sleep 12
  echo "add a health check endpoint"
  sleep 12
  echo "make it return json not plaintext"
  sleep 12
  echo "LGTM merge it"
  sleep 12
  echo "next"
  sleep 12
  echo "write a migration for the new schema"
  sleep 12
  echo "done?"
  sleep 12
  echo "/quit"
) | $BINARY chat 2>&1 | tee "$LOG_DIR/persona_a_output.txt"

echo ""
echo "  ✓ Persona A complete (25 turns)"
echo ""

# Snapshot Persona A's profile
if [ -f "$SOCIAL_DB" ]; then
  sqlite3 "$SOCIAL_DB" "SELECT profile_json FROM social_user_profile LIMIT 1;" > "$LOG_DIR/persona_a_profile.json" 2>/dev/null || echo "{}" > "$LOG_DIR/persona_a_profile.json"
  sqlite3 "$SOCIAL_DB" "SELECT COUNT(*) FROM social_evaluation_log;" > "$LOG_DIR/persona_a_eval_count.txt" 2>/dev/null || echo "0" > "$LOG_DIR/persona_a_eval_count.txt"
  sqlite3 "$SOCIAL_DB" "SELECT observation FROM social_observations;" > "$LOG_DIR/persona_a_observations.txt" 2>/dev/null || echo "" > "$LOG_DIR/persona_a_observations.txt"
  echo "  Evaluations run: $(cat $LOG_DIR/persona_a_eval_count.txt)"
else
  echo "  WARNING: social.db not found"
  echo "{}" > "$LOG_DIR/persona_a_profile.json"
fi

###############################################################################
# PERSONA B: Curious Student
###############################################################################
echo ""
echo "┌─ PERSONA B: Curious Student ─────────────────────────────┐"
echo "│  Style: polite, verbose, lots of questions, informal     │"
echo "│  Expected profile: low directness, high verbosity,       │"
echo "│                    low technical depth, patient pace      │"
echo "└──────────────────────────────────────────────────────────┘"
echo ""

# Reset for fresh persona
rm -f "$HOME/.temm1e/memory.db"
rm -f "$SOCIAL_DB"

(
  echo "hey! so i'm trying to learn about web development and i was wondering if you could help me understand how APIs work? like what exactly is a REST API?"
  sleep 12
  echo "oh cool thanks for explaining that! so when you say endpoints, does that mean like different URLs that do different things? can you give me an example?"
  sleep 12
  echo "that makes sense! but im a bit confused about something - whats the difference between GET and POST? i keep seeing both and im not sure when to use which one"
  sleep 12
  echo "ohhh okay i think i get it now! so GET is for reading data and POST is for sending new data right? what about PUT and DELETE? are those important too?"
  sleep 12
  echo "thanks so much for being patient with me haha. okay so i tried building a simple API with express.js but im getting a CORS error and i have no idea what that means. can you explain what CORS is and why its blocking my request?"
  sleep 12
  echo "ugh that sounds complicated. so i need to add some headers to my server? could you maybe show me step by step how to fix it? im using express if that helps"
  sleep 12
  echo "it worked!! thank you so much!! okay next question - i keep hearing about databases. should i use SQL or NoSQL? my friend says MongoDB is better but my professor says to learn SQL first. what do you think?"
  sleep 12
  echo "hmm interesting. i think ill start with SQL then. do you have any tips for a complete beginner learning SQL? like what should i focus on first?"
  sleep 12
  echo "this is really helpful! oh wait i also wanted to ask - what is authentication? like i know its about logging in but how does it actually work behind the scenes? how does the server know who i am?"
  sleep 12
  echo "wow sessions and tokens sound complicated! so JWT is like a passport that i carry around? thats a cool analogy. but what happens if someone steals my token? is that a security risk?"
  sleep 12
  echo "okay that makes me worried about security. what are the most common security mistakes beginners make? i really dont want to mess this up"
  sleep 12
  echo "thanks for the heads up on those! im writing all this down btw. okay so i started building a todo app (classic i know lol) and im stuck on how to connect my frontend to my backend. like how does the react app talk to the express server?"
  sleep 12
  echo "ohhh fetch API! ive heard of that. so i just call fetch with the URL of my server endpoint? do i need to do anything special with the data that comes back?"
  sleep 12
  echo "JSON.parse and all that... got it! hey quick question - what is async/await? i see it everywhere in javascript but i dont really understand why i need it"
  sleep 12
  echo "oh man promises are confusing. so async/await is like a nicer way to handle things that take time? like waiting for a server response? that actually makes more sense when you explain it that way"
  sleep 12
  echo "youre such a good teacher by the way! okay so im thinking about deploying my todo app. is heroku still a thing? where should a beginner deploy their first project?"
  sleep 12
  echo "vercel sounds easy! but i also want to learn about docker eventually. is docker hard to learn? my classmates talk about it but it seems intimidating"
  sleep 12
  echo "containers... so its like a virtual machine but lighter? i think i kind of get it. maybe ill tackle that after i finish the basics"
  sleep 12
  echo "good plan! oh one more thing - git and github. i know i should be using version control but honestly i find git confusing. like whats the difference between git add, git commit, and git push?"
  sleep 12
  echo "OH that staging area concept finally clicked! add to stage, commit to save locally, push to share. thank you! ive been so confused about that for weeks"
  sleep 12
  echo "hey so i was reading about design patterns and came across MVC. is that something i should learn as a beginner or is it too advanced?"
  sleep 12
  echo "okay so model view controller - the model is data, view is what users see, controller connects them? that actually sounds like what i was already doing without knowing it lol"
  sleep 12
  echo "haha yeah! okay i think ive asked you enough questions for today. this has been super helpful though! any final tips for a beginner web developer?"
  sleep 12
  echo "thanks so much for all the help! im feeling way more confident now. ill definitely come back with more questions later haha"
  sleep 12
  echo "bye! have a great day :)"
  sleep 12
  echo "/quit"
) | $BINARY chat 2>&1 | tee "$LOG_DIR/persona_b_output.txt"

echo ""
echo "  ✓ Persona B complete (25 turns)"
echo ""

# Snapshot Persona B's profile
if [ -f "$SOCIAL_DB" ]; then
  sqlite3 "$SOCIAL_DB" "SELECT profile_json FROM social_user_profile LIMIT 1;" > "$LOG_DIR/persona_b_profile.json" 2>/dev/null || echo "{}" > "$LOG_DIR/persona_b_profile.json"
  sqlite3 "$SOCIAL_DB" "SELECT COUNT(*) FROM social_evaluation_log;" > "$LOG_DIR/persona_b_eval_count.txt" 2>/dev/null || echo "0" > "$LOG_DIR/persona_b_eval_count.txt"
  sqlite3 "$SOCIAL_DB" "SELECT observation FROM social_observations;" > "$LOG_DIR/persona_b_observations.txt" 2>/dev/null || echo "" > "$LOG_DIR/persona_b_observations.txt"
  echo "  Evaluations run: $(cat $LOG_DIR/persona_b_eval_count.txt)"
else
  echo "  WARNING: social.db not found"
  echo "{}" > "$LOG_DIR/persona_b_profile.json"
fi

###############################################################################
# COMPARISON REPORT
###############################################################################
echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  Social Intelligence A/B Test — Results                     ║"
echo "╠══════════════════════════════════════════════════════════════╣"
echo "║                                                              ║"
echo "║  Persona A (Terse Tech Lead):                                ║"
echo "║    Evaluations: $(cat $LOG_DIR/persona_a_eval_count.txt 2>/dev/null || echo '?')                                              ║"
echo "║                                                              ║"
echo "║  Persona B (Curious Student):                                ║"
echo "║    Evaluations: $(cat $LOG_DIR/persona_b_eval_count.txt 2>/dev/null || echo '?')                                              ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
echo "Profiles saved to:"
echo "  $LOG_DIR/persona_a_profile.json"
echo "  $LOG_DIR/persona_b_profile.json"
echo ""
echo "Observations saved to:"
echo "  $LOG_DIR/persona_a_observations.txt"
echo "  $LOG_DIR/persona_b_observations.txt"
echo ""
echo "Full chat logs saved to:"
echo "  $LOG_DIR/persona_a_output.txt"
echo "  $LOG_DIR/persona_b_output.txt"

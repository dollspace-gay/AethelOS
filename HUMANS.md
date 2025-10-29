This document, humans.md, provides a guide for human developers to effectively collaborate with our AI coding agent, Claude. Following these principles will maximize the agent's performance, ensure code quality, and create a more efficient and predictable workflow.
The core philosophy is that the human acts as an architect and a guide. You don't need to write the code, but you must have a high-level understanding of the structure you are building. The AI is a brilliant but sometimes lazy programmer; it requires your guidance to stay on track and produce its best work.
The Core Workflow: Plan, Guide, Test, Correct

Our interaction with the agent follows a strict, iterative cycle for every task. The AI is more likely to make mistakes or become "lazy" when given large, complex instructions. By breaking work down, we ensure high-quality, verifiable output at every stage.

1. Plan and Decompose
Before writing any code, the first step is to create a detailed plan.
Break Down Epics into Phases: Take a large feature or task and break it down into logical phases.
Break Down Phases into Tiny Steps: Each phase must be further broken down into the smallest possible, verifiable steps. A good step is a single, clear action (e.g., "create the function signature for calculate_price," "add a new button to the UI," "write a unit test for the isValidEmail function").
Get AI Buy-in: Present this plan to the AI. This ensures the agent understands the full scope and sequence before beginning implementation.

2. Guide Step-by-Step
You will guide the agent through the plan one step at a time. Do not give it multiple steps at once.
Use Explicit Commands: Use a command-driven workflow to manage the process. Ensure your claude.md file contains definitions for commands that instruct the agent to proceed to the next step, test its work, or wait for your input.
The "Do, Test, Fix" Loop: For each tiny step, follow this strict sequence:
Do: Tell the agent to implement only the current step.
Test: Immediately instruct the agent to write and run a test to verify that the step was completed correctly. The agent must prove its work.
Fix: The AI often makes small errors. If the test fails, instruct the agent to fix the code it just wrote. Repeat the test until it passes.
Check for Laziness: Manually inspect the generated code for stubs, placeholders (// TODO:), or incomplete logic. AI agents can sometimes be lazy to save effort. Do not let it move on until the code is complete.
Only after a step has been implemented, tested, and verified should you instruct the agent to move to the next one.

Using beads for Task Management
To supercharge our planning and guidance workflow, we use beads, a lightweight, git-backed issue tracker designed specifically for AI coding agents.[1] beads provides a persistent, queryable memory that helps the agent manage long-term plans and automatically discover new work.[1][2] This solves the problem of context window limitations and agent amnesia.[1]
You, the human, do not typically interact with beads directly. Your role is to instruct the agent to use it.
Integration:
Installation: Ensure the bd command-line tool is installed in the development environment.
Configuration: Your claude.md file must instruct the agent to use the bd tool for all task and issue management, instead of markdown checklists.[1][3] A single line in claude.md is usually sufficient to point the agent to the tool.[3]

Workflow:
Initial Planning: When you provide the initial plan, instruct Claude to create an epic and file each step as an issue in beads.

Step-by-Step Execution: To begin work on a step, tell the agent to "start the next ready task in beads." The agent will query beads to find unblocked tasks.

Automatic Work Discovery: If the agent discovers new, unplanned work (e.g., a missing helper function, a needed refactor), it will automatically file new issues in beads and link them to the current task.[1][2] This prevents loss of work and context.[1]

Auditing and Resuming: The beads audit trail allows a new agent instance to quickly orient itself and pick up exactly where the last one left off.[1]

By using beads, we transform our manual checklists into a robust, graph-based dependency tracker that the AI can manage itself, freeing up the human guide to focus on higher-level strategy and code verification.

Context Management: When the Agent Fails

AI models can sometimes enter a bad state, especially during long or complex conversations. Recognizing and resetting this state is crucial.

Recognize the Signs: A key failure mode is when the agent begins outputting nonsensical or repetitive conversational filler, such as littering its responses with green checkmark emojis. If you see this, the agent is "cooked," and its context is likely corrupted.

The "Reboot" Procedure:
Open a New Context Window: Immediately stop interacting in the current conversation. Open a fresh session with the agent.
Re-read the Rules: Your first instruction should be to have it re-read the claude.md file. This re-establishes the project's rules, commands, and guidelines.
Resume from the Last Known Good State: Briefly summarize the overall goal and tell the agent exactly where you left off (e.g., "We are working on the user authentication feature. The beads tracker shows we just completed issue #12. Please start work on the next ready issue.").
Advanced Tactics
"Calling in the Advisor"
While Claude is our primary coding agent, it has weaknesses. Its general knowledge can be limited or outdated. Gemini, on the other hand, excels at research and high-level conceptual understanding.
Leverage Gemini for Research: If you or the agent are stuck on a high-level architectural problem, a complex algorithm, or a new library, use Gemini to ask for principles and explanations.
Feed the Knowledge to Claude: Take the clear, high-level explanation from Gemini and feed it to Claude as context to guide its implementation. This combines the research strength of one model with the coding strength of another.
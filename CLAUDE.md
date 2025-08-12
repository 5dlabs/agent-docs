
# Claude Code Project Memory

## Project Information
- **Repository**: 5dlabs/agent-docs
- **Source Branch**: main
- **GitHub App**: 5DLabs-Rex
- **Working Directory**: .
- **Implementation Target**: task 1

## Tool Capabilities

See @mcp-tools.md for your available tools and usage guidelines

## Project Guidelines & Standards

See @coding-guidelines.md for project coding standards and best practices
See @github-guidelines.md for git workflow and commit message standards

## Current Task Documentation

**Your current task (1) documentation:**
- See @task/task.md for requirements and description
- See @task/acceptance-criteria.md for success criteria
- See @task/architecture.md for technical approach and guidance

## System Architecture & Context

See @.taskmaster/docs/architecture.md for system design patterns and architectural decisions


## Implementation Workflow

### Current Task Process
1. **Understand**: Read @task/task.md for requirements
2. **Plan**: Review @task/architecture.md for technical approach
3. **Validate**: Check @task/acceptance-criteria.md for success criteria
4. **Code**: Follow patterns in @coding-guidelines.md
5. **Commit**: Use standards from @github-guidelines.md
6. **Test**: Verify all acceptance criteria are met

### Task Context
- **Task ID**: 1
- **Repository**: 5dlabs/agent-docs
- **Branch**: main
- **Working Directory**: .

## Quick Command Reference

### Testing & Quality
```bash
# Run tests (check package.json/Cargo.toml for project-specific commands)
npm test || cargo test

# Linting and formatting
npm run lint || cargo clippy
npm run format || cargo fmt

# Build verification
npm run build || cargo build
```

### Git Workflow
```bash
# Commit with task-specific message (see @github-guidelines.md for details)
git commit -m "feat(task-1): implement [brief description]

- [specific changes made]
- [tests added/updated]
- [meets acceptance criteria: X, Y, Z]"
```

## Pull Request Requirements

**CRITICAL**: After completing implementation, create `PR_DESCRIPTION.md` in the working directory root with:

1. Concise implementation summary (2-3 sentences)
2. Key changes made (bullet points)
3. Important reviewer notes
4. Testing recommendations

This file enables automatic pull request creation.

## Development Tools & Patterns

### Claude Code Integration
- Use `LS` and `Glob` to explore codebase structure
- Use `Read` to examine existing code patterns
- Use `Grep` to find similar implementations
- Use `Edit` for targeted changes, `MultiEdit` for related changes
- Validate with `Bash` commands after each change

### Implementation Guidelines
- Focus on current task requirements in `task/` directory
- Follow architectural guidance provided in @task/architecture.md
- Ensure all acceptance criteria are met before completion
- Use established patterns from @coding-guidelines.md

---

*All referenced files (@filename) are automatically imported into Claude's context. For detailed information on any topic, refer to the specific imported files above.*

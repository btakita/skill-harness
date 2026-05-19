# Install Skill

Procedure for installing an AI agent skill into a project.

## Prerequisites

- `skill-harness` binary available (via `cargo install` or PATH)
- Source SKILL.md file exists

## Steps

1. **Identify the skill source**
   - Locate the SKILL.md file to install
   - Verify it has valid content (title heading, description)

2. **Detect the environment**
   - skill-harness auto-detects Claude Code, OpenCode, Codex, Cursor, or Generic
   - Override with `--root` flag if needed

3. **Run install**
   ```bash
   skill-harness install <name> --file <path/to/SKILL.md>
   ```
   For full skill directories, use:
   ```bash
   skill-harness install-dir <name> --source <path/to/skill-directory> --harness <target>
   ```
   For the bundled compose-skills package, omit `--source`:
   ```bash
   skill-harness install-dir compose-skills --harness codex
   ```

4. **Verify installation**
   ```bash
   skill-harness check <name> --file <path/to/SKILL.md>
   ```
   For full skill directories, use:
   ```bash
   skill-harness check-dir <name> --source <path/to/skill-directory> --harness <target>
   ```
   - Should report "Up to date"

5. **Commit the installed skill**
   - The installed file should be committed to version control
   - `git add <installed-path> && git commit -m "Install <name> skill"`

## Verification

- [ ] `skill-harness check` reports "Up to date"
- [ ] Installed file exists at the correct environment-specific path
- [ ] File content matches the source SKILL.md

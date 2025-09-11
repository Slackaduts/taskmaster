# Taskmaster
A task automation framework to make writing maintainable, documented scripts easier for nontechnical people.

## Features
- Aliased/sanitized tasks defined through YAML to eliminate user error
- System-wide reporting to a JSON trace via HTTP
- Documentation generation to Markdown via CLI with examples
- Userspace by default unless absolutely neccessary; Ensures scripts are not needlessely run with administrator privileges

## Usage
Type `.\taskmaster.exe --help` to get started.

## Immediate plans
- Limit tokio features
- Concurrent tasks

## Future plans
- Unix support (Bash)
- Variable initialization
- Conditionals outside of raw PowerShell

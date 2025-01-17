# Korah: A CLI Utility for Natural Language Queries

Korah is a powerful yet user-friendly command-line utility that simplifies complex tasks using natural language queries. Currently, it includes tools for file searching and process monitoring, with plans to expand its capabilities in the future.

## Currently Supported Tools

### **Find Files**
Quickly locate files and directories on your local file system using flexible search criteria:
- **Name patterns**: Match files or directories by name.
- **Content patterns**: Search within file contents using regular expressions.
- **Search directory**: Specify the directory to search in.
- **File type**: Filter by files, directories, or symlinks.
- **Size range**: Define minimum or maximum file sizes.
- **Timestamps**: Filter by creation or modification time within a specified range.

### **Find Processes**
Easily filter and monitor running processes on your operating system, with options for detailed or summarized output:
- **Name patterns**: Match processes by name using regular expressions.
- **CPU usage**: Specify minimum or maximum CPU consumption percentages.
- **Memory usage**: Filter by RAM usage range.
- **Disk I/O**: Set limits on data read from or written to disk.
- **Network ports**: Filter processes using specific TCP or UDP ports.

## Examples

```sh
korah 'find videos on the desktop'
{"path":"/Users/john.smith/Desktop/foo.mkv"}
{"path":"/Users/john.smith/Desktop/bar.avi"}
```

```sh
korah 'find processes with "gram" in name'
{"name":"Telegram","pid":25537}
```

## Installation

1. Run `cargo install korah`.
2. Copy `korah.toml` into `~/.config`.

### With Ollama LLM Backend

3. Install [Ollama](https://ollama.com/).
4. Choose and install a model to be used, e.g. `qwen2.5`.
5. Make sure the `ollama` LLM API and the model are configured in `korah.toml`.

### With OpenAI LLM Backend

3. Make sure the `open_ai` LLM API, model and key are configured in `korah.toml`.

# TimeShine

A high-performance, lightweight, and reliable backup utility written in Rust. It features full multi-architecture support and is designed to execute fast backup operations with modern concurrency paradigms.

## Features

- **Blazing Fast**: Optimized backup execution utilizing Rust's concurrency and safety features.
- **Multi-Architecture Support**: Built to support multiple CPU architectures efficiently.
- **Lightweight**: Minimal dependencies, ensuring speed and stability.

## Installation

### Building from Source

To build and install TimeShine from source, ensure you have Rust and Cargo installed, then run the following commands:

```bash
git clone https://github.com/ryusawawa/timeshine.git
cd timeshine
cargo install --path .
```

## Usage
### ​1. Basic Operations
Backing Up Data
​To create a backup of a specific directory or file, use the backup subcommand. You must specify the source <src> and the destination <dest>:

```bash
timeshine backup /path/to/source /path/to/destination
```
Restoring Data
To restore data from an existing backup, use the restore subcommand. Specify the backup source and the target directory where you want the files restored:
```bash
timeshine restore /path/to/backup /path/to/restore_target
```
### 2. Full System Backup (System-wide)
To back up the entire system, execute TimeShine with root privileges (sudo). It is highly recommended to exclude virtual filesystems (such as /proc, /sys, /dev, and /run) to avoid infinite loops and system errors.

Executing Full System Backup
Run the following command to secure your entire root directory:
```bash
sudo timeshine backup / /path/to/secure_external_storage
```
​⚠️ Note: Ensure your destination drive has enough capacity and is properly mounted before running a full system backup.

Restoring Full System
​To restore the entire system from a root backup, boot into a live environment, mount your target partition, and execute:
```bash
sudo timeshine restore /path/to/backup /mnt/target_root
```
### 3. Help and Options
To view the complete list of available subcommands, flags, and detailed options, run the help command:
```bash
timeshine --help
```







## License and Copyright

This project is licensed under the MIT License.

```text
Copyright (c) 2026 ryusawawa

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
## Contact
If you have any questions, bug reports, or feature requests, feel free to open an issue or reach out via email:

GitHub: ryusawawa
Email: kaitoubanana7@gmail.com



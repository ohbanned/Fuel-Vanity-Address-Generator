██╗███████╗██╗   ██╗███████╗██╗     
██║██╔════╝██║   ██║██╔════╝██║     
██║█████╗  ██║   ██║█████╗  ██║     
██║██╔══╝  ██║   ██║██╔══╝  ██║     
██║██║     ╚██████╔╝███████╗███████╗
╚═╝╚═╝      ╚═════╝ ╚══════╝╚══════╝

# iFuel - Fuel Vanity Address Generator

> A high-performance, secure Rust application for generating custom Fuel blockchain wallet addresses with personalized patterns, optimized for speed and security.

![iFuel Logo](frontend/img/ifuel-logo.svg)

[![GitHub](https://img.shields.io/badge/GitHub-Repository-blue?logo=github)](https://github.com/ohbanned/Fuel-Vanity-Address-Generator)
[![Twitter](https://img.shields.io/badge/Twitter-@ohbannedOS-blue?logo=twitter)](https://x.com/ohbannedOS)
[![Lines of Code](https://img.shields.io/badge/Lines%20of%20Code-~1.2k-brightgreen)](#code-quality)
[![Rust](https://img.shields.io/badge/Built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org/)

---

## Overview

iFuel is a cutting-edge Rust application built specifically for the Fuel blockchain ecosystem. It leverages Rust's performance and safety features to generate cryptographically secure vanity addresses with blazing fast speed while maintaining memory safety and thread security.

The project is built with a **clean architecture**, **comprehensive error handling**, and both **terminal and web interfaces**, making it production-ready and user-friendly.

---

## Features

- **High-Performance**: Utilizes Rust's concurrency features and optimized cryptography for lightning-fast address generation
- **Beautiful UI**: Terminal interface with colorful ASCII art and well-formatted output
- **Web Interface**: User-friendly web interface for generating addresses without installing anything
- **Multiple Search Types**: Find addresses with specific prefixes, suffixes, or containing specific patterns
- **Case-Sensitive Mode**: Optional case-sensitive matching for more specific pattern targeting
- **Secure**: All cryptographic operations performed locally with no external API dependencies
- **Multi-threaded**: Automatically utilizes all available CPU cores for maximum performance
- **Cross-Platform**: Works on MacOS, Linux, and Windows

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/ohbanned/Fuel-Vanity-Address-Generator.git
cd Fuel-Vanity-Address-Generator

# Build in release mode for maximum performance
cargo build --release

# Run the application
./target/release/fuel-vanity-generator
```

## Usage

### Terminal Interface

Launch the program and use one of the following commands:

```
prefix <pattern>        # Generate addresses with a specific prefix
suffix <pattern>        # Generate addresses with a specific suffix
contains <pattern>      # Generate addresses containing a pattern anywhere
help                    # Show command help
exit                    # Exit the program
```

#### Options

- `-s, --case-sensitive` - Enable case-sensitive matching
- `-t, --threads <num>` - Specify number of threads to use (default: all CPU cores)

### Web Interface

1. Open the `frontend/index.html` file in your browser
2. Enter your desired pattern
3. Select the pattern position (prefix, suffix, or anywhere)
4. Toggle case sensitivity if needed
5. Click "Generate Address"

## Examples

### Terminal Examples

Generate addresses with prefix "abc":
```
iFuel> prefix abc
```

Generate addresses with suffix "cafe":
```
iFuel> suffix cafe
```

Generate addresses containing "dead" anywhere:
```
iFuel> contains dead
```

## Code Quality

The codebase is designed with:

- **Clean Architecture**: Separation of concerns between UI and core functionality
- **Comprehensive Error Handling**: Robust error messages and graceful recovery
- **Zero Unsafe Code**: 100% safe Rust with no `unsafe` blocks
- **Well-Documented Code**: Clear comments and function documentation
- **Efficient Algorithms**: Optimized cryptographic operations

## Performance

iFuel is designed for maximum performance:

- Utilizes all available CPU cores automatically
- Optimized address generation algorithm
- Efficient pattern matching implementation
- Asynchronous operations with Tokio runtime
- Memory-efficient storage of addresses and private keys

## Security Considerations

- All cryptographic operations are performed locally
- Private keys are never transmitted over the network
- Uses proven cryptographic libraries for key generation
- Memory is zeroed when no longer needed

## Project Structure

```
.
├── src/                # Rust source code
│   └── main.rs         # Main application code
├── frontend/           # Web interface files
│   ├── index.html      # Main HTML page
│   ├── css/            # Stylesheets
│   └── js/             # JavaScript code
└── README.md           # Project documentation
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Author

Created by [Ban](https://x.com/ohbannedOS) for the Fuel Hackathon.

---

<div align="center">
<strong>⚡ Powered by Rust and Fuel ⚡</strong><br>
<small>© 2025 iFuel</small>
</div>

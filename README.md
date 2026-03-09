<div align="center">
  <img src="src-tauri/icons/128x128.png" alt="JD Launcher Icon" width="128" />
  <h1>JD Launcher (under development)</h1>
  <p>A modern, lightweight, and blazingly fast Minecraft launcher built with Rust and React.</p>
</div>

---

## 🚀 Overview

**JD Launcher** is a modern, high-performance Minecraft launcher featuring deep Modrinth integration, Microsoft account support, and advanced instance management. Built with Tauri, Rust, and React, it provides a native feel while remaining cross-platform compatible.

### ✨ Features
- **🚀 Blazingly Fast**: Minimal resource footprint and fast startup thanks to Tauri and Rust.
- **🔐 Microsoft Integration**: Seamless Xbox/Microsoft account authentication and management.
- **📦 Modrinth Browser**: Search and install modpacks, mods, and more directly within the app.
- **🛠️ Instance Control**: Full management of profiles, including duplication, importing/exporting, and content toggling.
- **☕ Auto-Java**: Automatic detection and downloading of version-specific Java runtimes.
- **🎨 Modern Aesthetic**: Polished UI built with React, Tailwind CSS v4, and shadcn/ui.
- **🔗 Type-Safe**: Robust communication between frontend and backend via Specta.

## 🛠️ Built With

- **[Tauri 2.0](https://tauri.app/)** - Secure, lightweight native apps
- **[Rust](https://www.rust-lang.org/)** - Backend safety and performance
- **[React 18](https://react.dev/)** - Component-based UI formulation
- **[Vite](https://vitejs.dev/)** - Next-generation frontend tooling
- **[Tailwind CSS v4](https://tailwindcss.com/)** - Utility-first CSS framework
- **[shadcn/ui](https://ui.shadcn.com/)** - Beautifully designed system components
- **[Specta](https://github.com/oscartbeaumont/specta)** - Painless, type-safe Rust to TS bindings

## ⚙️ Getting Started

### Prerequisites
Make sure you have the following installed to run and build JD Launcher:
1. [Rust](https://www.rust-lang.org/tools/install)
2. [Bun](https://bun.sh/) (or Node.js/npm)
3. Essential Tauri build dependencies. See [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/).

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/xxeisenberg/jd-launcher.git
   cd jd-launcher
   ```

2. Install frontend dependencies:
   ```bash
   bun install
   ```

3. Run the development server:
   ```bash
   bun run tauri dev
   ```

### Building for Production
To build a production-ready application bundle:
```bash
bun run tauri build
```
Compiled binaries will be available in `src-tauri/target/release/bundle`.

## 🤝 Contributing
Contributions are always welcome! Feel free to open an issue or submit a Pull Request.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

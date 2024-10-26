[![Rust](https://github.com/amicloud/SealSlicer/actions/workflows/rust.yml/badge.svg)](https://github.com/amicloud/SealSlicer/actions/workflows/rust.yml)
# SealSlicer

SealSlicer is a **work-in-progress** slicer tailored specifically for MSLA resin 3D printing. Designed with both performance and usability in mind.

*This file last updated on October 14, 2024*

---

## 🛠️ Current Progress

### ✔️ Mesh Rendering Engine
- Fully implemented and operational.
- Provides accurate and efficient rendering of 3D meshes.
- Currently using a basic Physically Based Renderering shader
- Really need to add an orthographic view and a button to change between ortho and perspective

### 🖥️ CPU-only Slicer

- **Status:** Working
- Able to slice most objects correctly. 
- Generates image data that will be usable by a future g-code/printer file creator and ctb sdk
- Has some bugs where some polygons in a slice are not rendered properly -- weird issues affecting <1% of slice images
- Fully multithreaded, will use 100% of each core currently

### 🛠️ Printer Settings Module

- **Status:** Basic Functionality Implemented.
  - Load printer settings from `.toml` files for easy configuration, customization, and sharing.
- **Planned Functionality:**
  - GUI for creating and editing printer profiles
  - Some way to actually select a printer, it is currently hardcoded

  ### 🫱🏿‍🫲🏻 User Settings Module

- **Status:** Basic Functionality Implemented.
  - Load user settings from `.toml` files for easy configuration, customization, and sharing.
- **Planned Functionality:**
  - GUI for creating and editing profiles
  - Some way to actually select a profile, it is currently hardcoded

### 🖥️ Highly Responsive UI

- **Status:** Implemented.
- Slicing is currently blocking the UI while saving the debug images to the drive. :shrug:

### 🔄 Multithreading

- **Status:** Looking Good.
- Actively working on leveraging multithreading to enhance performance wherever beneficial.
- CPU Slicer is able to utilize all cpu resources. It's actually a bit of a hog but really fast. Might need to configure niceness/priority somehow

---

## 🚀 Upcoming Features

### 📝 G-code/3D Printer File Generator

- **Status:** Not started.
- **Planned Functionality:**
  - Generate G-code files compatible with MSLA resin printers

### 🖼️ G-code/Slice Images Viewer

- **Status:** Not started.
- **Planned Functionality:**
  - Visualize G-code and slice images within the application for better inspection and verification before printing.

### 🎛️ GPU Compute Slicer

- **Status:** Not working
- Needs to be updated to be in line with the way the CPU slicer works. Now that the CPU slicer is mostly working this might come soon

### 🖧 Network Printing

- **Status:** Not started.
- **Planned Functionality:**
  - Send files for printing directly to printers over the network.
  - This might actually not be possible with the current Chitubox SDK. 

---

## 📊 Test Coverage

Trying to ensure robust and reliable functionality through comprehensive testing. Currently focusing on components not involving OpenGL.

- **Total Coverage:** **36.19%** 485/1340 lines covered

## 🌟 Goals

SealSlicer aims to provide a comprehensive and efficient slicing solution for MSLA resin printers with the following objectives:

- **Mesh Rendering Engine:** Accurate and efficient rendering of 3D meshes.
- **CPU-only Slicer:** Normal slicing functionality using CPU resources.
- **GPU Compute Slicer:** Enhanced slicing performance leveraging GPU acceleration.
- **Printer Settings Module:** Easy configuration through `.toml` files.
- **G-code/3D Printer File Generator:** Direct generation of printer-compatible files.
- **G-code/Slice Images Viewer:** Visual inspection of slicing results.
- **High Responsiveness:** Ensuring the UI remains responsive during intensive operations.
- **Multithreading:** Utilizing multiple threads to optimize performance.

---

## 🤝 Contributing

SealSlicer is currently not fleshed out enough for meaningful contributions. Contributions are welcome once the core functionalities are stable and generalized. Stay tuned for updates!

If you're interested in contributing, please follow these guidelines:

1. **Fork the Repository:** Create your own fork of SealSlicer.
2. **Create a Feature Branch:** Branch off from `main` to work on your feature or bug fix.
3. **Commit Your Changes:** Ensure your commits are descriptive.
4. **Open a Pull Request:** Submit your changes for review. Please only submit properly formatted code. Use cargo fmt

Feel free to reach out with suggestions or if you're interested in collaborating!

---

## 📝 Notes

- **GPU Slicing:** The GPU slicing component is broken currently
- **Responsive UI:** While the UI is currently responsive, slicing operations block the UI thread. Fixes are planned to resolve this.
- **Test Coverage:** Focused on ensuring reliability for non-OpenGL components, with ongoing efforts to increase coverage.
- **Future Enhancements:** Plans include expanding compatibility, and adding user-friendly features like settings management and file visualization.


---

## 📝 License

This project is licensed under the **GNU Affero General Public License v3.0**.

You are free to run, study, modify, and share this software under the terms of the AGPL-3.0, with the added requirement that if the software is used to interact with users over a network, the source code must be made available to those users.

### Key License Points:
- You may **use**, **modify**, and **share** this software.
- If you make modifications and provide it over a network the source code must be made available

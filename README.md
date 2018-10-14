# aircode-client

Unofficial third-party client for [Codea](http://codea.io)'s Air Code server. This project is not affiliated with Codea. Use at your own risk!

## Installation

 * Install `cargo`, the Rust package manager, from [rustup.rs](https://rustup.rs) or otherwise.
 * Run `cargo build --release && cargo install`.

## Instructions

 * Start the Air Code server from the Codea app.
 * Run `aircode-client`.
 * When prompted, enter the project name you wish to edit (press tab for auto-completion).
 * A new directory called `project` will be created with all the project files inside.
 * The client is now running and will auto-update files:
   * Edit project files under the `project/` directory and save to update.
   * Run `touch project/restart` to restart your project.
   * Press `Ctrl+C` to exit the client.

## Re-installation

 * Run `cargo install --force`.

## Uninstall

 * Run `cargo uninstall`.

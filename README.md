# Nvim Switcher
This is a simple program written in Rust that allows you to easily switch between different versions of neovim.
The version is pulled from Neovim's [Github](https://github.com/neovim/neovim/releases).

## Prerequisites
- This only works on Linux.
- You must have added your ~/.local/bin directory to your PATH, since this is where neovim will be installed.

## Installation
Simply place the executable in your PATH and you're good to go.

## Usage
There are several commands you can use with this program:
- `nvim_switcher current` shows the currently installed version
- `nvim_switcher switch VERSION` switch the currently installed version to the specified version
- `nvim_switcher download VERSION` download the specified version of neovim, but do not install it
- `nvim_switcher purge VERSION` remove a downloaded version of neovim (this will not uninstall the currently installed version, but instead simply remove the download)

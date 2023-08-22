# ramp - rust another music player

Ramp is a no bullshit, batteries included, configurable music player for the terminal. It is designed to be lightweight, customizable and easy to use.

> **Note:** This project is still a work in progress, contributions are welcome.

## Features

Ramp supports basically all common audio formats thanks to [symphonia](https://crates.io/crates/symphonia).

It also uses caching to store metadata about music files in order to avoid loading times during use.

Other than that, it basically just plays music.

## Installation

`cargo install --path .`
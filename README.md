## dynlock

Dynamic and Configurable Lockscreen with Customizable UI and Shader Support

This tool's foundation builds heavily on the concepts and design
in [RobinMcCorkell's Shaderlock](https://github.com/RobinMcCorkell/shaderlock)
so credit to him for his original creation.

### Features
  - Blazingly fast 🔥
  - Simple and easy to use
  - Shader support for infinite possibilities
  - Uses the official [ext-session-lock-v1 protocol](https://wayland.app/protocols/ext-session-lock-v1)
  - Comes installed with an assortment of pretty shaders

### Installation

Install build dependencies (Ubuntu)

```bash
$ sudo apt install build-essential make cmake pkg-config llvm libclang-dev libpam-dev libxkbcommon-dev
```

Compile and install binaries

```bash
$ make install
```

### Usage

Run it with ease!

```
$ dynlock
```

View all available options via the built-in help:

```bash
$ dynlock --help
```

### Screenshots

#### Frost

![frost](./screenshots/frost.png)

#### Paper Burn

![paper-burn](./screenshots/paper-burn.png)

#### Floating Orb

![orb](./screenshots/orb.png)

#### Matrix

![matrix](./screenshots/matrix.png)

## dynlock

Dynamic and configurable lockscreen with customizable UI and shader support

This tools foundation builds heavily on the concepts and design
in [RobinMcCorkell's Shaderlock](https://github.com/RobinMcCorkell/shaderlock)
so credit to him for his original creation.

### Features
  - Blazingly Fast 🔥
  - Simple and Easy to Use
  - Shader Support for Infinite Possibilities
  - Uses The Official [ext-session-lock-v1 protocol](https://wayland.app/protocols/ext-session-lock-v1)
  - Comes Installed with an Assortment of Pretty Shaders

### Installation

Install Build Dependencies (Ubuntu)

```bash
$ sudo apt install build-essential make cmake pkg-config llvm libclang-dev libpam-dev libxkbcommon-dev
```

Compile and Install Binaries

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

# McMultiplayerController
## Description
This was a project intended for my friends to be able to start my minecraft server. It also served as a way for me to learn async Rust and gRPC.
The program is built so that a less powerhungry computer can recieve server start commands and then start up a more powerful host that can run a minecraft server.

The rust language was chosen since I wanted to become better at the language and wanted the program to have a small memory- and cpu footprint.

## Structure
The project two subprogrammes:
* orchestrator - Is used for the end user to input commands, eg. "/start_server". This program is also responsible for starting the host (via wake-on-lan) which runs the mchostd program.
* mchostd - Is ran on the host which should start and manage the minecraft servers. Recieves commands from the orchestrator.

## Environment variables
Environment variables are loaded in from a .env file in the root of the project. Environment variables that are set before running the program take precedence over variables set in the .env file.

## discord_dot
Contains both a gRPC server for sending commands to the minecraft server host and a discord bot to recieve commands from the user.

## Running
```bash
$ cargo run --bin orchestrator 
```

### Environment variables
The available environment variables are written in orchestrator/constants.rs

## mchostd
When started it tries to connect to the gRPC server of the orchestrator.

Features:
* Starts/stops minecraft servers
* Automatically stops minecraft servers when all players leave
* Automatically shutsdown the computer which the program is running on when all minecraft servers have stopped and no one is using the computer 
* Create minecraft servers
* Aware of different versions of minecraft server software

### Installation
You have to create your own systemd service and move the binary to the correct place 

### Running
```bash
$ cargo run --bin mchostd 
```

### Environment variables
Found user mchost/constants

# McMultiplayerController
## Description
This was a project intended for my friends to be able to start my minecraft server. It also served as a way for me to learn async Rust and gRPC.
The programme is built so that a less powerhungry computer can recieve server start commands and then start up a more powerfull host that can run a minecraft server.

For this small personal project bugs were acceptable and the project just needed to be done as soon as possible, since I was going away for a while and would not be able to finish the project for a couple of weeks. Thus the code is not refactored and the code is, at some places horribly nested to be able to not worry about fighting the borrow checker.

## Structure
The project contains three subprogrammes:
* discord_bot - Is used for the end user to input commands, eg. "/start_server"
* mchostd - Is the daemon that recieves commands from the discord_bot via gRPC.
* mchost - Is used to configure the daemon.

## Environment variables
Environment variables are loaded in from a .env file in the root of the project. Environment variables that are set before running the program take precedence over variables set in the .env file.

## Discord_dot
Contains both a gRPC server for sending commands to the minecraft server host and a discord bot to recieve commands from the user.

## Running
```bash
$ cargo run --bin discord_bot 
```

### Environment variables
```.env
discord_token
guild_id # The guild where the bot will register and listen for commands
wol_mac # The mac addres of the host computers eth device, used for triggering a Wake On Lan when the server should start
listen_address # Which address the gPRC server should bind to
```

## mchostd
When started it tries to connect to the gRPC server of the discord bot.

It will automatically stop the server if no one has been on the server for 30s.

### Running
```bash
$ cargo run --bin mchostd 
```

### Environment variables
```.env
controller_address # The address of the discord_bot gRPC server, eg. http://192.168.1.223:50051
```

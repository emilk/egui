Supports the ability to render egui remotely using eframe.

These examples use tungstenite (https://github.com/snapview/tungstenite-rs) for setting up the websocket client and server, but any setup should work.
Server
- Receives connections and information from client
- Specifies egui content, resulting in a FullOutput
- Serializes the FullOutput and sends to client
Client
- Connects to server
- Sends user input information to server
- Receives serialized content from server, deserializes it and displays it in an eframe

```sh
cargo r -p remote_rendering --bin server
cargo r -p remote_rendering --bin client "ws://localhost:8083"
```

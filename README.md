# 128H-Final-Project - Ektarit Gus Nophaket (gusn2)


# Project Introduction
I am planning to develop a simple client-server chat application in Rust with a sleek graphical user interface. This application will allow users to connect to a server, join different chat rooms to exchange messages, and send direct messages to other users. The goal is to explore networking in a world of digital communication.


# Technical Overview
Crates
- `Tokio`. This will be used to manage asynchronous tasks throughout the project. It will handle the core networking tasks like establishing connections, concurrently processing messages, and scheduling tasks efficiently.
- `Iced`. This will provide the framework used for the user interface. It allows us to build a user interface that seamlessly adapts to user interaction and real-time updates.
  
Components
- Server. Listens on a TCP port and uses Tokio to accept connections. Receives messages and determines their recipients. Keeps track of connected users and chatrooms.
- Client. A desktop application built with Iced. Displays chatrooms, other users, and messages in real time. Sends and receives messages from the server to update GUI.

Roadmap
- Checkpoint 1: Basic networking and server setup.
- Checkpoint 2: Chatrooms, direct messaging, and server improvements.
- Checkpoint 3 (Final): Building the frontend GUI.

# Possible Challenges
- Learning crates
- Developing an aesthetically pleasing and fully functional user interface
- Handling asynchronous networking
- Integrating the front and backend seamlessly, especially when using new technologies/crates

# References
Inspiration
- https://github.com/nag763/tchatchers?tab=readme-ov-file
- https://github.com/barafael/achat
- https://github.com/Yengas/rust-chat-server
- https://github.com/jshaughnessy24/project_cs128h?tab=readme-ov-file

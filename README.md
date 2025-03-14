# Hecate Server

## Configuration

* Modify `backend/Rocket.toml` to match your environment for:
  * server IP and port

## Building and running

* Install the rust `wasm32` target and the `trunk` web-application bundler:
    ```bash
    rustup target add wasm32-unknown-unknown
    cargo install trunk
    ```

* Build the web-application:
  ```bash
  cd ui
  trunk build --release
  cd ..
  ```

* Run the server:
  ```bash
  cd backend
  cargo run
  ```

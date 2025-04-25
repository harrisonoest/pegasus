# Project Structure for Pegasus

```
pegasus/
├── Cargo.toml
├── Dockerfile
├── src/
│   ├── main.rs              # Application entry point
│   ├── config.rs            # Configuration management
│   ├── downloader.rs        # Video downloading logic
│   ├── processor.rs         # Media processing logic
│   ├── transfer.rs          # Media server transfer logic
│   ├── api/
│   │   ├── mod.rs           # API module exports
│   │   ├── routes.rs        # API route definitions
│   │   └── handlers.rs      # API request handlers
│   └── web/
│       ├── mod.rs           # Web module exports
│       └── server.rs        # Web server configuration
├── static/                  # Static frontend files
│   ├── index.html           # Main page
│   ├── css/
│   │   └── styles.css       # CSS styles
│   └── js/
│       └── app.js           # Frontend JavaScript
└── .dockerignore            # Files to exclude from Docker build
```
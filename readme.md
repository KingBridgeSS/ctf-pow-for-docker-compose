# CTF-PoW-for-docker-compose

A proof-of-work wrapper for CTF challenges hosted with docker-compose, inspired by it's singleton-container version: [pow-wrapper](https://github.com/mnixry/pow-wrapper).

-  For player

`python3 client.py`

- For host

Replace your docker-compose.yml's port mapping with `{{port}}` and rename it into docker-compose.tpl  (see ./example)

```
Usage: ctf-pow-for-docker-compose [OPTIONS] --compose-dir <COMPOSE_DIR>

Options:
      --compose-dir <COMPOSE_DIR>          The directory containing the docker-compose.tpl file
      --port <PORT>                        The port to listen on [default: 1337]
      --difficulty <DIFFICULTY>            The difficulty of the proof of work [default: 6]
      --pow-timeout <POW_TIMEOUT>          The timeout for the proof of work (seconds) [default: 30]
      --service-timeout <SERVICE_TIMEOUT>  The timeout for the service (seconds) [default: 120]
  -h, --help                               Print help
```


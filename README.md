[//]: # SPDX-FileCopyrightText: © Matteo Settenvini <matteo.settenvini@montecristosoftware.eu>
[//]: # SPDX-License-Identifier: EUPL-1.2

# serves3

A **very** simple proxy to browse files from private S3 buckets.

Helpful to be put behind another authenticating web server, such as Apache or NGINX.

Also helpful to do a different TLS termination.

## Configuration

Copy `Settings.toml.example` to `Settings.toml` and adjust your settings.

You can also add a `Rocket.toml` file to customize the server options. See the [Rocket documentation](https://rocket.rs/v0.5-rc/guide/configuration/#rockettoml).

Then just configure Apache or NGINX to proxy to the given port. For example:

```apache
<VirtualHost *:443>
    ServerName example.com
    ServerAdmin support@example.com
    DocumentRoot /var/www

    ProxyPreserveHost On
    ProxyPass /s3/ http://127.0.0.1:8000/
    ProxyPassReverse /s3/ http://127.0.0.1:8000/

    # ... other options ...
</VirtualHost>
```

You probably also want a systemd unit file, for instance `/etc/systemd/system/serves3@.service`:

```ini
[Unit]
Description=ServeS3, a S3 proxy
StartLimitInterval=100
StartLimitBurst=10

[Service]
Type=simple
ExecStart=/usr/local/bin/serves3
WorkingDirectory=/etc/serves3/%i/
Environment=ROCKET_PORT=%i

Restart=always
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

Then, e.g. for running on port 8000, you would put the corresponding configuration file in `/etc/serves3/8000/` and install the unit with `systemctl enable --now serves3@8000.service`.

## Build and install

If you want more granular control on installation options, use CMake:

```bash
cmake -B build .
cmake --build build
cmake --install build
cd run-folder # folder with Settings.toml
serves3
```

Else you can simply rely on `cargo`:

```bash
cargo install --root /usr/local --path . # for instance
cd run-folder # folder with Settings.toml
serves3
```

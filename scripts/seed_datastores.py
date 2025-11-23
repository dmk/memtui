#!/usr/bin/env python3
"""
Seed Redis and Memcached datastores with sample data.

This script populates Redis and Memcached instances with realistic sample data
for testing and development purposes.
"""

import argparse
import json
import socket
import sys
import time

try:
    import redis
    from faker import Faker
    from tqdm import tqdm
    from colorama import Fore, Style, init as colorama_init
    colorama_init(autoreset=True)
except ImportError as e:
    print(f"Error: {e}", file=sys.stderr)
    print("Please install required packages:", file=sys.stderr)
    print("  pip install -r scripts/requirements.txt", file=sys.stderr)
    sys.exit(1)


class DatastoreSeeder:
    """Manages seeding of Redis and Memcached datastores."""

    # Default base counts for various data types
    BASE_REDIS_USERS = 200
    BASE_MEMCACHED_USERS = 120
    BASE_JOBS = 40
    BASE_EVENT_BUCKETS = 5
    BASE_LEADERBOARD_ENTRIES = 4
    BASE_EMAIL_QUEUE = 3
    BASE_ORGS = 12
    BASE_PROJECTS_PER_ORG = 4
    BASE_PRODUCTS = 30
    BASE_CARTS = 45
    BASE_NOTIFICATIONS = 120
    BASE_AUDIT_EVENTS = 200
    BASE_SERVICE_INSTANCES = 12
    BASE_API_TOKENS = 80
    BASE_RATE_LIMIT_ENTRIES = 140
    BASE_DOCUMENTS = 90
    BASE_DASHBOARDS = 25
    BASE_WEBHOOKS = 16
    BASE_SUPPORT_TICKETS = 60
    BASE_RELEASES = 20

    def __init__(
        self,
        redis_host: str = "127.0.0.1",
        redis_port: int = 6379,
        memcached_host: str = "127.0.0.1",
        memcached_port: int = 11211,
        scale: float = 1.0,
        seed: int = None,
        flush: bool = False,
        verbose: bool = False,
    ):
        self.redis_host = redis_host
        self.redis_port = redis_port
        self.memcached_host = memcached_host
        self.memcached_port = memcached_port
        self.scale = scale
        self.flush = flush
        self.verbose = verbose

        # Calculate scaled counts
        self.num_users = int(self.BASE_REDIS_USERS * scale)
        self.num_memcached_users = int(self.BASE_MEMCACHED_USERS * scale)
        self.num_jobs = int(self.BASE_JOBS * scale)
        self.num_event_buckets = max(1, int(self.BASE_EVENT_BUCKETS * scale))
        self.num_leaderboard_entries = max(1, int(self.BASE_LEADERBOARD_ENTRIES * scale))
        self.num_email_queue = max(1, int(self.BASE_EMAIL_QUEUE * scale))
        self.num_orgs = max(1, int(self.BASE_ORGS * scale))
        self.num_projects_per_org = max(1, int(self.BASE_PROJECTS_PER_ORG * scale))
        self.num_products = max(5, int(self.BASE_PRODUCTS * scale))
        self.num_carts = max(5, int(self.BASE_CARTS * scale))
        self.num_notifications = max(10, int(self.BASE_NOTIFICATIONS * scale))
        self.num_audit_events = max(10, int(self.BASE_AUDIT_EVENTS * scale))
        self.num_service_instances = max(2, int(self.BASE_SERVICE_INSTANCES * scale))
        self.num_api_tokens = max(10, int(self.BASE_API_TOKENS * scale))
        self.num_rate_limits = max(10, int(self.BASE_RATE_LIMIT_ENTRIES * scale))
        self.num_documents = max(10, int(self.BASE_DOCUMENTS * scale))
        self.num_dashboards = max(5, int(self.BASE_DASHBOARDS * scale))
        self.num_webhooks = max(3, int(self.BASE_WEBHOOKS * scale))
        self.num_support_tickets = max(5, int(self.BASE_SUPPORT_TICKETS * scale))
        self.num_releases = max(5, int(self.BASE_RELEASES * scale))

        # Setup Faker
        self.fake = Faker()
        if seed is not None:
            Faker.seed(seed)

    def log(self, message: str, level: str = "info"):
        """Print colored log message."""
        colors = {
            "info": Fore.CYAN,
            "success": Fore.GREEN,
            "warning": Fore.YELLOW,
            "error": Fore.RED,
            "header": Fore.MAGENTA,
        }
        color = colors.get(level, Fore.WHITE)
        prefix = f"{color}▸{Style.RESET_ALL}"
        print(f"{prefix} {message}")

    def print_header(self, text: str):
        """Print a fancy header."""
        width = 60
        print()
        print(f"{Fore.MAGENTA}{'═' * width}{Style.RESET_ALL}")
        print(f"{Fore.MAGENTA}{text.center(width)}{Style.RESET_ALL}")
        print(f"{Fore.MAGENTA}{'═' * width}{Style.RESET_ALL}")
        print()

    def memcached_set(self, sock: socket.socket, key: str, value, ttl: int = 1800):
        """Helper to set a value in Memcached."""
        if not isinstance(value, str):
            value = json.dumps(value)
        payload = value.encode()
        command = f"set {key} 0 {ttl} {len(payload)}\r\n".encode()
        sock.sendall(command + payload + b"\r\n")
        sock.recv(1024)

    def seed_redis(self):
        """Populate Redis with sample data."""
        self.print_header("REDIS DATASTORE")

        self.log(f"Connecting to Redis at {Fore.YELLOW}{self.redis_host}:{self.redis_port}{Style.RESET_ALL}")

        # Connect to Redis
        try:
            r = redis.Redis(
                host=self.redis_host,
                port=self.redis_port,
                decode_responses=True,
                socket_connect_timeout=5,
            )
            r.ping()
            self.log("Connected successfully", "success")
        except redis.ConnectionError as e:
            self.log(f"Failed to connect: {e}", "error")
            raise

        # Flush all existing data (if requested)
        if self.flush:
            self.log("Flushing existing data", "warning")
            r.flushall()
        else:
            self.log("Skipping flush (use --flush to clear existing data)")

        # Basic configuration
        self.log("Creating configuration entries")
        pipe = r.pipeline()
        pipe.set("app:config:version", "1.3.0")
        pipe.hset("app:config:flags", mapping={
            "readonly": "true",
            "telemetry_enabled": "true",
            "theme": "nord",
        })

        # Email queue with realistic emails
        email_types = ["welcome", "receipt", "digest", "notification"]
        for _ in range(self.num_email_queue):
            email_type = self.fake.random_element(email_types)
            username = self.fake.user_name()
            pipe.lpush("queue:emails", f"{email_type}:{username}")

        # Feature flags
        pipe.sadd("feature:flags", "dark_mode", "live_reload", "beta_banner")

        # Leaderboard with realistic names
        leaderboard_data = {}
        for _ in range(self.num_leaderboard_entries):
            name = self.fake.first_name().lower()
            score = self.fake.random_int(min=800, max=1300)
            leaderboard_data[name] = score
        for name, score in leaderboard_data.items():
            pipe.zadd("leaderboard", {name: score})

        # Event streams
        pipe.xadd("streams:events", {
            "type": "signup",
            "user_id": "1010",
            "plan": "pro",
        })
        pipe.xadd("streams:events", {
            "type": "plan_change",
            "user_id": "1001",
            "plan": "enterprise",
        })

        pipe.execute()

        # Sessions and users with progress bar
        self.log(f"Creating {Fore.YELLOW}{self.num_users}{Style.RESET_ALL} users with sessions")
        tiers = ["free", "pro", "enterprise"]

        bar_color = "\033[36m"  # Cyan
        for i in tqdm(
            range(1, self.num_users + 1),
            desc=f"{bar_color}Users{Style.RESET_ALL}",
            unit="user",
            bar_format="{l_bar}{bar}| {n_fmt}/{total_fmt} [{elapsed}<{remaining}]",
        ):
            # Determine tier
            if i % 3 == 0:
                tier = "enterprise"
            elif i % 2 == 0:
                tier = "pro"
            else:
                tier = "free"

            # Create session
            session_token = self.fake.uuid4()
            r.set(f"session:{i}", session_token)

            # Create user with realistic data
            r.hset(f"user:{i}", mapping={
                "name": self.fake.name(),
                "email": self.fake.email(),
                "tier": tier,
                "joined": self.fake.date_time_this_year().isoformat(),
                "last_login": self.fake.date_time_this_month().isoformat(),
            })

        # Event buckets
        events_per_bucket = max(1, int(20 * self.scale))
        total_events = self.num_event_buckets * events_per_bucket
        self.log(f"Creating {Fore.YELLOW}{total_events}{Style.RESET_ALL} events across {Fore.YELLOW}{self.num_event_buckets}{Style.RESET_ALL} buckets")

        for bucket in tqdm(
            range(self.num_event_buckets),
            desc=f"{bar_color}Event buckets{Style.RESET_ALL}",
            unit="bucket",
            bar_format="{l_bar}{bar}| {n_fmt}/{total_fmt}",
        ):
            for event in range(1, events_per_bucket + 1):
                timestamp = int(time.time()) - self.fake.random_int(min=0, max=86400)
                r.lpush(f"events:{bucket}", f"{timestamp}-event-{bucket}-{event}")

        # Service registry and health checks
        self.log(f"Registering {Fore.YELLOW}{self.num_service_instances}{Style.RESET_ALL} service instances")
        services = ["api", "jobs", "worker", "scheduler", "web", "metrics"]
        regions = ["us-west-1", "us-east-1", "eu-central-1", "ap-southeast-1"]
        for instance in range(1, self.num_service_instances + 1):
            service_name = self.fake.random_element(services)
            instance_id = f"{service_name}:{instance:03d}"
            mapping = {
                "service": service_name,
                "region": self.fake.random_element(regions),
                "host": f"{service_name}-{instance}.svc.local",
                "status": self.fake.random_element(["passing", "warning", "critical"]),
                "started": self.fake.date_time_this_month().isoformat(),
            }
            r.hset(f"service:{instance_id}", mapping=mapping)
            r.sadd(f"service:index:{service_name}", instance_id)
            r.zadd("services:uptime", {instance_id: self.fake.random_int(min=900, max=86400)})

        # Organizations and projects
        self.log(f"Creating {Fore.YELLOW}{self.num_orgs}{Style.RESET_ALL} organizations with projects")
        industry_tags = ["fintech", "gaming", "saas", "health", "education", "iot"]
        for org_id in range(1, self.num_orgs + 1):
            org_key = f"org:{org_id:03d}"
            org_mapping = {
                "name": f"{self.fake.company()}",
                "plan": self.fake.random_element(["starter", "growth", "enterprise"]),
                "industry": self.fake.random_element(industry_tags),
                "owner": self.fake.email(),
                "created_at": self.fake.date_time_this_year().isoformat(),
            }
            r.hset(org_key, mapping=org_mapping)
            for project_idx in range(1, self.num_projects_per_org + 1):
                slug = f"proj-{org_id:03d}-{project_idx:02d}"
                project_key = f"{org_key}:project:{project_idx:02d}"
                r.hset(project_key, mapping={
                    "slug": slug,
                    "status": self.fake.random_element(["active", "maintenance", "archived"]),
                    "region": self.fake.random_element(regions),
                    "last_deploy": self.fake.date_time_this_month().isoformat(),
                })
                r.sadd(f"{org_key}:projects", slug)
                r.zadd("projects:activity", {slug: self.fake.random_int(min=1, max=1000)})
                r.lpush(f"{project_key}:activity", self.fake.sentence(nb_words=6))

        # Product catalog and inventory
        self.log(f"Publishing {Fore.YELLOW}{self.num_products}{Style.RESET_ALL} catalog items")
        categories = ["observability", "productivity", "security", "billing", "messaging"]
        for product_id in range(1, self.num_products + 1):
            sku = f"sku-{product_id:04d}"
            price = round(self.fake.pyfloat(left_digits=3, right_digits=2, positive=True, min_value=9, max_value=499), 2)
            stock = self.fake.random_int(min=1, max=250)
            r.hset(f"catalog:product:{product_id:04d}", mapping={
                "sku": sku,
                "name": self.fake.catch_phrase(),
                "category": self.fake.random_element(categories),
                "price": f"{price:.2f}",
                "currency": "USD",
                "updated": self.fake.date_time_this_month().isoformat(),
            })
            r.zadd("catalog:prices", {sku: price})
            r.hset("inventory:levels", sku, stock)
            if stock < 10:
                r.sadd("inventory:low_stock", sku)

        # Shopping carts
        self.log(f"Building {Fore.YELLOW}{self.num_carts}{Style.RESET_ALL} shopping carts")
        for cart_id in range(1, self.num_carts + 1):
            item_count = self.fake.random_int(min=1, max=4)
            items = []
            for _ in range(item_count):
                sku = f"sku-{self.fake.random_int(min=1, max=self.num_products):04d}"
                qty = self.fake.random_int(min=1, max=3)
                items.append({"sku": sku, "qty": qty})
                r.hincrby("inventory:reserved", sku, qty)
            r.hset(f"cart:{cart_id:04d}", mapping={
                "user_id": str(self.fake.random_int(min=1, max=self.num_users)),
                "items": json.dumps(items),
                "updated": self.fake.date_time_this_month().isoformat(),
            })
            r.zadd("cart:abandoned", {f"cart:{cart_id:04d}": self.fake.random_int(min=1, max=1000)})

        # Notifications and channels
        self.log(f"Streaming {Fore.YELLOW}{self.num_notifications}{Style.RESET_ALL} notifications")
        notification_types = ["deployment", "invoice", "alert", "comment", "handoff"]
        channels = ["email", "slack", "pagerduty", "sms"]
        for _ in range(self.num_notifications):
            user_id = self.fake.random_int(min=1, max=self.num_users)
            payload = {
                "type": self.fake.random_element(notification_types),
                "channel": self.fake.random_element(channels),
                "message": self.fake.sentence(nb_words=8),
                "created": self.fake.date_time_this_month().isoformat(),
            }
            stream_key = f"stream:notifications:{user_id:04d}"
            r.xadd(stream_key, payload)
            r.hincrby("notifications:unread", str(user_id), 1)
            r.sadd(f"user:{user_id}:channels", payload["channel"])

        # Audit events
        self.log(f"Appending {Fore.YELLOW}{self.num_audit_events}{Style.RESET_ALL} audit events")
        audit_actions = ["login", "logout", "plan_change", "role_update", "api_access", "mfa_challenge"]
        for _ in range(self.num_audit_events):
            event = {
                "user": self.fake.random_int(min=1, max=self.num_users),
                "action": self.fake.random_element(audit_actions),
                "ip": self.fake.ipv4_public(),
                "ts": int(time.time()) - self.fake.random_int(min=0, max=604800),
                "success": self.fake.boolean(chance_of_getting_true=92),
            }
            r.lpush("audit:trail", json.dumps(event))

        # API tokens and rate limits
        self.log(f"Issuing {Fore.YELLOW}{self.num_api_tokens}{Style.RESET_ALL} API tokens")
        for _ in range(self.num_api_tokens):
            user_id = self.fake.random_int(min=1, max=self.num_users)
            token = self.fake.uuid4().replace("-", "")
            ttl = self.fake.random_int(min=3600, max=3600 * 24 * 14)
            r.set(f"api:token:{token}", str(user_id), ex=ttl)
            r.sadd("api:tokens:index", token)

        self.log(f"Configuring {Fore.YELLOW}{self.num_rate_limits}{Style.RESET_ALL} rate limit counters")
        endpoints = ["/v1/metrics", "/v1/events", "/v1/users", "/v1/search", "/v1/tokens"]
        for _ in range(self.num_rate_limits):
            endpoint = self.fake.random_element(endpoints)
            user_id = self.fake.random_int(min=1, max=self.num_users)
            limit_key = f"ratelimit:{user_id}:{endpoint}"
            r.hset(limit_key, mapping={
                "minute": self.fake.random_int(min=0, max=120),
                "hour": self.fake.random_int(min=0, max=500),
                "day": self.fake.random_int(min=0, max=3000),
                "window_start": self.fake.date_time_this_month().isoformat(),
            })

        # Documents and dashboards
        self.log(f"Publishing {Fore.YELLOW}{self.num_documents}{Style.RESET_ALL} knowledge base docs")
        for doc_id in range(1, self.num_documents + 1):
            tags = self.fake.words(nb=3)
            body = "\n".join(self.fake.paragraphs(nb=50))
            doc_key = f"doc:{doc_id:04d}"
            r.set(doc_key, body)
            r.sadd("doc:index", doc_key)
            for tag in tags:
                r.sadd(f"doc:tag:{tag}", doc_key)

        self.log(f"Designing {Fore.YELLOW}{self.num_dashboards}{Style.RESET_ALL} dashboards")
        themes = ["nord", "gruvbox", "light", "solarized"]
        for dash_id in range(1, self.num_dashboards + 1):
            definition = {
                "widgets": self.fake.random_int(min=3, max=9),
                "theme": self.fake.random_element(themes),
                "layout": self.fake.random_element(["grid", "stack", "freeform"]),
                "refresh_interval": self.fake.random_int(min=10, max=120),
            }
            r.hset(f"dashboard:{dash_id:03d}", mapping={
                "owner_org": str(self.fake.random_int(min=1, max=self.num_orgs)),
                "definition": json.dumps(definition),
                "updated": self.fake.date_time_this_month().isoformat(),
            })

        # Webhooks and releases
        self.log(f"Scheduling {Fore.YELLOW}{self.num_webhooks}{Style.RESET_ALL} webhook deliveries")
        for hook_id in range(1, self.num_webhooks + 1):
            payload = {
                "delivery": f"hook-{hook_id:05d}",
                "status": self.fake.random_element(["pending", "sent", "errored"]),
                "ms": str(self.fake.random_int(min=35, max=1200)),
            }
            r.xadd("stream:webhooks", payload)
            r.zadd("webhooks:latency", {payload["delivery"]: float(payload["ms"])})

        self.log(f"Queuing {Fore.YELLOW}{self.num_releases}{Style.RESET_ALL} releases")
        environments = ["dev", "staging", "prod"]
        for release_id in range(1, self.num_releases + 1):
            release = {
                "id": f"rel-{release_id:04d}",
                "environment": self.fake.random_element(environments),
                "version": f"{self.fake.random_int(1, 3)}.{self.fake.random_int(0, 20)}.{self.fake.random_int(0, 99)}",
                "owner": self.fake.email(),
            }
            r.rpush("deploy:pipelines", json.dumps(release))
            r.xadd("streams:deployments", {
                "release": release["id"],
                "env": release["environment"],
                "status": self.fake.random_element(["queued", "running", "complete"]),
            })

        # Support tickets
        self.log(f"Tracking {Fore.YELLOW}{self.num_support_tickets}{Style.RESET_ALL} support tickets")
        ticket_statuses = ["new", "triaged", "in_progress", "waiting", "closed"]
        priorities = ["low", "medium", "high", "urgent"]
        for ticket_id in range(1, self.num_support_tickets + 1):
            status = self.fake.random_element(ticket_statuses)
            ticket_key = f"support:ticket:{ticket_id:05d}"
            r.hset(ticket_key, mapping={
                "subject": self.fake.sentence(nb_words=5),
                "status": status,
                "priority": self.fake.random_element(priorities),
                "requester": self.fake.email(),
                "created": self.fake.date_time_this_year().isoformat(),
            })
            if status != "closed":
                r.sadd("support:open", ticket_key)

        # Analytics and metrics
        self.log("Populating analytics artifacts")
        for _ in range(self.num_users * 3):
            r.pfadd("analytics:unique_visitors", self.fake.uuid4())
        for user_id in range(1, self.num_users + 1):
            if user_id % 4 == 0:
                r.setbit("analytics:active_bitmap", user_id, 1)
        for channel in channels:
            r.hset("analytics:channel_usage", channel, self.fake.random_int(min=10, max=500))

        # Geospatial warehouse index
        self.log("Indexing warehouse locations")
        warehouses = ["us-west", "us-east", "eu-north", "apac"]
        # Predefined coordinates for warehouses to ensure valid ranges
        warehouse_coords = {
            "us-west": (-122.4194, 37.7749),  # San Francisco
            "us-east": (-74.0060, 40.7128),   # New York
            "eu-north": (18.0686, 59.3293),   # Stockholm
            "apac": (151.2093, -33.8688),     # Sydney
        }
        for warehouse in warehouses:
            longitude, latitude = warehouse_coords.get(warehouse, (self.fake.longitude(), self.fake.latitude()))
            # Use execute_command to avoid nx/xx option conflicts in geoadd method
            r.execute_command("GEOADD", "geo:warehouses", longitude, latitude, warehouse)
            r.hset(f"warehouse:{warehouse}", mapping={
                "capacity": self.fake.random_int(min=50, max=500),
                "manager": self.fake.name(),
                "updated": self.fake.date_time_this_year().isoformat(),
            })

        self.log("Redis seeding complete", "success")

    def seed_memcached(self):
        """Populate Memcached with sample data."""
        self.print_header("MEMCACHED DATASTORE")

        self.log(f"Connecting to Memcached at {Fore.YELLOW}{self.memcached_host}:{self.memcached_port}{Style.RESET_ALL}")

        # Connect to Memcached
        try:
            sock = socket.create_connection(
                (self.memcached_host, self.memcached_port),
                timeout=5,
            )
            self.log("Connected successfully", "success")
        except (socket.error, socket.timeout) as e:
            self.log(f"Failed to connect: {e}", "error")
            raise

        try:
            # Flush all existing data (if requested)
            if self.flush:
                self.log("Flushing existing data", "warning")
                sock.sendall(b"flush_all\r\n")
                sock.recv(1024)
            else:
                self.log("Skipping flush (use --flush to clear existing data)")

            # Users with progress bar
            self.log(f"Creating {Fore.YELLOW}{self.num_memcached_users}{Style.RESET_ALL} users")
            bar_color = "\033[36m"  # Cyan
            for i in tqdm(
                range(1, self.num_memcached_users + 1),
                desc=f"{bar_color}Users{Style.RESET_ALL}",
                unit="user",
                bar_format="{l_bar}{bar}| {n_fmt}/{total_fmt} [{elapsed}<{remaining}]",
            ):
                segment = "enterprise" if i % 2 == 1 else "consumer"

                # Generate realistic user data
                user_data = {
                    "id": i,
                    "name": self.fake.name(),
                    "segment": segment,
                    "email": self.fake.email(),
                    "company": self.fake.company() if segment == "enterprise" else None,
                }

                # Remove None values
                user_data = {k: v for k, v in user_data.items() if v is not None}

                key = f"users:{i:03d}"
                self.memcached_set(sock, key, user_data, ttl=1800)

            # Jobs with progress bar
            self.log(f"Creating {Fore.YELLOW}{self.num_jobs}{Style.RESET_ALL} jobs")
            states = ["scheduled", "running", "finished"]
            for i in tqdm(
                range(1, self.num_jobs + 1),
                desc=f"{bar_color}Jobs{Style.RESET_ALL}",
                unit="job",
                bar_format="{l_bar}{bar}| {n_fmt}/{total_fmt}",
            ):
                state = states[i % 3]
                duration = 20 + i

                value = f"job-{i:02d}|state={state}|duration={duration}s"
                key = f"jobs:{i:02d}"
                self.memcached_set(sock, key, value, ttl=600)

            # Metrics
            self.memcached_set(sock, "metrics:ingest", "0", ttl=3600)
            self.memcached_set(sock, "metrics:alerts", "0", ttl=3600)

            # App config with realistic region
            regions = ["us-west-1", "us-west-2", "us-east-1", "eu-west-1", "ap-southeast-1"]
            envs = ["dev", "staging", "prod"]
            region = self.fake.random_element(regions)
            env = self.fake.random_element(envs)
            config_value = f"region={region}|env={env}"
            self.memcached_set(sock, "app:config", config_value, ttl=3600)

            # Session cache
            self.log(f"Caching {Fore.YELLOW}{self.num_users}{Style.RESET_ALL} session artifacts")
            for i in tqdm(
                range(1, self.num_users + 1),
                desc=f"{bar_color}Sessions{Style.RESET_ALL}",
                unit="session",
                bar_format="{l_bar}{bar}| {n_fmt}/{total_fmt}",
            ):
                session = {
                    "user_id": i,
                    "client": self.fake.random_element(["web", "mobile", "cli"]),
                    "ip": self.fake.ipv4_public(),
                    "token": self.fake.uuid4(),
                    "expires_at": self.fake.date_time_this_month().isoformat(),
                }
                ttl = self.fake.random_int(min=900, max=7200)
                self.memcached_set(sock, f"cache:session:{i:04d}", session, ttl=ttl)

            # Experiment assignments
            self.log("Storing experiment assignments")
            experiments = ["onboarding_flow", "search_v2", "beta_nav", "billing_banner"]
            for idx in range(1, self.num_users + 1):
                assignment = {exp: self.fake.random_element(["control", "variant"]) for exp in experiments}
                self.memcached_set(sock, f"cache:experiments:{idx:04d}", assignment, ttl=14400)

            # Notification digests
            self.log("Creating notification digests")
            for idx in range(1, self.num_notifications + 1):
                digest = {
                    "count": self.fake.random_int(min=1, max=12),
                    "channels": self.fake.random_elements(elements=["email", "slack", "sms"], length=2, unique=False),
                    "generated": self.fake.date_time_this_month().isoformat(),
                }
                self.memcached_set(sock, f"cache:notifications:{idx:04d}", digest, ttl=3600)

            # Inventory snapshots
            self.log("Caching inventory snapshots")
            for product_id in range(1, self.num_products + 1):
                snapshot = {
                    "sku": f"sku-{product_id:04d}",
                    "stock": self.fake.random_int(min=0, max=250),
                    "reserved": self.fake.random_int(min=0, max=25),
                    "updated": self.fake.date_time_this_month().isoformat(),
                }
                self.memcached_set(sock, f"cache:inventory:{product_id:04d}", snapshot, ttl=300)

            # Dashboard renders
            self.log("Preparing dashboard render cache")
            for dash_id in range(1, self.num_dashboards + 1):
                render = {
                    "dashboard_id": dash_id,
                    "widgets": [self.fake.word() for _ in range(self.fake.random_int(min=3, max=7))],
                    "generated": self.fake.date_time_this_month().isoformat(),
                }
                self.memcached_set(sock, f"cache:dashboard:{dash_id:03d}", render, ttl=600)

            # Org usage reports
            self.log("Caching org usage reports")
            for org_id in range(1, self.num_orgs + 1):
                report = {
                    "org_id": org_id,
                    "events": self.fake.random_int(min=1000, max=50000),
                    "errors": self.fake.random_int(min=0, max=500),
                    "spend": round(self.fake.pyfloat(left_digits=4, right_digits=2, positive=True, min_value=120, max_value=9000), 2),
                }
                self.memcached_set(sock, f"cache:org_report:{org_id:03d}", report, ttl=1800)

            # Feature flag snapshots
            self.log("Caching feature flag snapshots")
            flag_variants = ["enabled", "disabled", "beta"]
            for flag in ["dark_mode", "live_reload", "new_console", "ai_assist"]:
                payload = {
                    "flag": flag,
                    "value": self.fake.random_element(flag_variants),
                    "updated": self.fake.date_time_this_month().isoformat(),
                }
                self.memcached_set(sock, f"cache:feature:{flag}", payload, ttl=120)

            # Search suggestions
            self.log("Seeding search suggestions")
            keywords = ["error", "latency", "timeout", "deployment", "billing"]
            for keyword in keywords:
                suggestions = [self.fake.word() for _ in range(5)]
                self.memcached_set(sock, f"cache:search:{keyword}", "|".join(suggestions), ttl=900)

            # Pricing cache
            self.log("Caching pricing tiers")
            tiers = ["free", "pro", "enterprise", "scale", "edge"]
            for tier in tiers:
                price = round(self.fake.pyfloat(left_digits=3, right_digits=2, positive=True, min_value=1, max_value=250), 2)
                details = f"tier={tier}|price={price}|currency=USD"
                self.memcached_set(sock, f"pricing:{tier}", details, ttl=7200)

            # Report exports
            self.log("Caching report exports")
            for idx in range(1, self.num_jobs + 5):
                export = {
                    "job": f"export-{idx:04d}",
                    "owner": self.fake.email(),
                    "format": self.fake.random_element(["csv", "json", "parquet"]),
                    "rows": self.fake.random_int(min=100, max=5000),
                }
                self.memcached_set(sock, f"cache:export:{idx:04d}", export, ttl=5400)

            # Quit
            sock.sendall(b"quit\r\n")

            self.log("Memcached seeding complete", "success")
        finally:
            sock.close()

    def run(self):
        """Run the full seeding process."""
        try:
            # Print welcome banner
            print()
            print(f"{Fore.CYAN}╔═══════════════════════════════════════════════════════════╗")
            print(f"{Fore.CYAN}║           DATASTORE SEEDING UTILITY                       ║")
            print(f"{Fore.CYAN}╚═══════════════════════════════════════════════════════════╝{Style.RESET_ALL}")
            print()

            # Show configuration
            self.log(f"Scale factor: {Fore.YELLOW}{self.scale}x{Style.RESET_ALL}")
            if self.scale != 1.0:
                self.log(f"  Redis users: {Fore.YELLOW}{self.num_users}{Style.RESET_ALL} (base: {self.BASE_REDIS_USERS})")
                self.log(f"  Memcached users: {Fore.YELLOW}{self.num_memcached_users}{Style.RESET_ALL} (base: {self.BASE_MEMCACHED_USERS})")
                self.log(f"  Jobs: {Fore.YELLOW}{self.num_jobs}{Style.RESET_ALL} (base: {self.BASE_JOBS})")
            print()

            # Seed both datastores
            self.seed_redis()
            self.seed_memcached()

            # Print success summary
            print()
            print(f"{Fore.GREEN}╔═══════════════════════════════════════════════════════════╗")
            print(f"{Fore.GREEN}║                    SEEDING COMPLETE                       ║")
            print(f"{Fore.GREEN}╚═══════════════════════════════════════════════════════════╝{Style.RESET_ALL}")
            print()
            self.log(f"Total Redis keys: {Fore.YELLOW}~{self.num_users * 2 + 50}{Style.RESET_ALL}")
            self.log(f"Total Memcached keys: {Fore.YELLOW}~{self.num_memcached_users + self.num_jobs + 3}{Style.RESET_ALL}")
            print()

        except KeyboardInterrupt:
            print()
            self.log("Interrupted by user", "warning")
            sys.exit(130)
        except Exception as e:
            print()
            self.log(f"Error: {e}", "error")
            if self.verbose:
                raise
            sys.exit(1)


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Seed Redis and Memcached datastores with sample data",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )

    parser.add_argument(
        "--redis-host",
        default="127.0.0.1",
        help="Redis host address",
    )
    parser.add_argument(
        "--redis-port",
        type=int,
        default=6379,
        help="Redis port",
    )
    parser.add_argument(
        "--memcached-host",
        default="127.0.0.1",
        help="Memcached host address",
    )
    parser.add_argument(
        "--memcached-port",
        type=int,
        default=11211,
        help="Memcached port",
    )
    parser.add_argument(
        "--scale",
        type=float,
        default=1.0,
        help=(
            "Scale multiplier for data generation (default: 1.0). "
            f"Base amounts: {DatastoreSeeder.BASE_REDIS_USERS} Redis users, "
            f"{DatastoreSeeder.BASE_MEMCACHED_USERS} Memcached users, "
            f"{DatastoreSeeder.BASE_JOBS} jobs. "
            "Use 0.5 for half, 2.0 for double, etc."
        ),
    )
    parser.add_argument(
        "--seed",
        type=int,
        help="Random seed for reproducible data generation",
    )
    parser.add_argument(
        "--flush",
        action="store_true",
        help="Flush all existing data before seeding (WARNING: destructive operation)",
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Enable verbose output",
    )

    args = parser.parse_args()

    seeder = DatastoreSeeder(
        redis_host=args.redis_host,
        redis_port=args.redis_port,
        memcached_host=args.memcached_host,
        memcached_port=args.memcached_port,
        scale=args.scale,
        seed=args.seed,
        flush=args.flush,
        verbose=args.verbose,
    )

    seeder.run()


if __name__ == "__main__":
    main()


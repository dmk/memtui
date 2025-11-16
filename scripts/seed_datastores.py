#!/usr/bin/env python3
"""
Seed Redis and Memcached datastores with sample data.

This script populates Redis and Memcached instances with realistic sample data
for testing and development purposes.
"""

import argparse
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

                import json
                value = json.dumps(user_data)
                key = f"users:{i:03d}"
                cmd = f"set {key} 0 1800 {len(value)}\r\n{value}\r\n"
                sock.sendall(cmd.encode())
                sock.recv(1024)

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
                cmd = f"set {key} 0 600 {len(value)}\r\n{value}\r\n"
                sock.sendall(cmd.encode())
                sock.recv(1024)

            # Metrics
            sock.sendall(b"set metrics:ingest 0 3600 1\r\n0\r\n")
            sock.recv(1024)
            sock.sendall(b"set metrics:alerts 0 3600 1\r\n0\r\n")
            sock.recv(1024)

            # App config with realistic region
            regions = ["us-west-1", "us-west-2", "us-east-1", "eu-west-1", "ap-southeast-1"]
            envs = ["dev", "staging", "prod"]
            region = self.fake.random_element(regions)
            env = self.fake.random_element(envs)
            config_value = f"region={region}|env={env}"
            cmd = f"set app:config 0 3600 {len(config_value)}\r\n{config_value}\r\n"
            sock.sendall(cmd.encode())
            sock.recv(1024)

            # Config
            self.log("Creating application config")

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


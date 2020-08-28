# yam
A simple application that notifies me of some very basic information via slack.

So far this has only two abilities: 
- connect to a set of URLs and log the status code
- connect to a postgres database, run a set of queries that return a single integer value and log the return value

The app will log to stdout and to a slack url.

To configure the app, the YAM_CONFIG_FILE environment variable has to be set to the location of a toml file

Sample config file:
```
DATABASE_URL = "postgres://pg_user:pg_pass@127.0.0.1:5432/main_db"
SLACK_URL = "https://hooks.slack.com/services/xxxx/yyyyy/aaaa" # obtained when setting up a slack application
SLEEP_SECONDS = 15 # how often the database queries and URLs are checked. if no changes are detected from the last time the queries are run, slack won't be notified. 
RESEND_MINUTES = 60 # slack is always notified of the current state every RESEND_MINUTES
MONITOR_URLS = ["https://www.example.com/heartbeat",
  "https:://www.example2.com/heartbeat"]
DB_QUERIES = [
    ["user count", "select count(*) as count from users"],
    ["login attemps", "select count(*) as count from login_attempts"]] # array of ["query name", "query body"]
```

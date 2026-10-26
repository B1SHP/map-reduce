# Lists the recipes
default:
    @just --list

# Builds the handlers
build:
    cargo build --release --bins

# Seed hadoop
seed:
    hdfs dfs -mkdir -p /data/input.csv
    hdfs dfs -put -f d /data/input.csv /data/input.csv 

hadoop_root := env_var_or_default("HADOOP_ROOT", "/data/forest-fires")

default:
    @just --list

# Builds release binaries
build:
    cargo build --release --bins

# Runs whole pipeline locally
run-local: build
    cat data/input.csv | target/release/mapper | target/release/combiner | target/release/reducer

# Sends complete input file to HDFS
hdfs-upload:
    hdfs dfs -mkdir -p {{hadoop_root}}/input
    hdfs dfs -put -f data/input.csv {{hadoop_root}}/input/input.csv

# Submits Hadoop Streaming job; output directory must not already exist
hadoop-run streaming_jar: build
    hadoop jar {{streaming_jar}} -files target/release/mapper,target/release/combiner,target/release/reducer -input {{hadoop_root}}/input -output {{hadoop_root}}/output -mapper mapper -combiner combiner -reducer reducer

# Shows Hadoop job output after job creates output directory
hdfs-output:
    hdfs dfs -cat {{hadoop_root}}/output/part-*

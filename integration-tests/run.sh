#!/bin/sh
set -e

ARG_DEFAULT_SUDO=
ARG_DEFAULT_SKIP_SETUP=
ARG_DEFAULT_SKIP_SC_COMP=
ARG_DEFAULT_SKIP_GENDATA=
ARG_DEFAULT_SKIP_CLEANUP=
ARG_DEFAULT_TESTS="test_rpc"
# ARG_DEFAULT_TESTS="test_rpc test_bus-mapping"

usage() {
    cat >&2 << EOF
        Usage: $0 [OPTIONS]
        Options:
          --sudo         Use sudo for docker-compoes commands.
          --skip-setup   Skip setting up docker containers.
          --skip-sc-comp Skip Smart Contracts compilation.
          --skip-gendata Skip generating blockhain data.
          --skip-cleanup Skip cleaning up docker containers.
          --tests ARG    Space separated list of tests to run.
                         Default: "${TEST_ARG_DEFAULT}".
          -h | --help    Show help

EOF
}

ARG_SUDO="${ARG_DEFAULT_SUDO}"
ARG_SKIP_SETUP="${ARG_DEFAULT_SKIP_SETUP}"
ARG_SKIP_SC_COMP="${ARG_DEFAULT_SKIP_SC_COMP}"
ARG_SKIP_GENDATA="${ARG_DEFAULT_SKIP_GENDATA}"
ARG_SKIP_CLEANUP="${ARG_DEFAULT_SKIP_CLEANUP}"
ARG_TESTS="${ARG_DEFAULT_TESTS}"

while [ "$1" != "" ]; do
    case $1 in
        --sudo )
            ARG_SUDO=1
        ;;
        --skip-setup )
            ARG_SKIP_SETUP=1
        ;;
        --skip-sc-comp )
            ARG_SKIP_SC_COMP=1
        ;;
        --skip-gendata )
            ARG_SKIP_GENDATA=1
        ;;
        --skip-cleanup )
            ARG_SKIP_CLEANUP=1
        ;;
        --tests )
            shift
            ARG_TESTS="$1"
        ;;
        -h | --help )    usage
            exit
        ;;
        * )              usage
            exit 1
    esac
    shift
done

docker_compose_cmd() {
    if [ -n $ARG_SUDO ]; then
        sudo docker-compose $@
    else
        docker-compose $@
    fi
}

if [ -z $ARG_SKIP_SETUP ]; then
    echo "+ Setup..."
    docker_compose_cmd down -v --remove-orphans
    docker_compose_cmd up -d geth0
fi

if [ -z $ARG_SKIP_SC_COMP ]; then
    echo "+ Smart Contracts compilation..."
    echo "TODO"
    # cd contracts
    # solc *
    # cd ..
fi

if [ -z $ARG_SKIP_GENDATA ]; then
    echo "+ Gen blockchain data..."
    # TODO: Delete output.json
    cargo run --bin gen_blockchain_data
fi

for testname in $ARG_TESTS; do
    echo "+ Running test group $testname"
    cargo test --features $testname
done

if [ -z $ARG_SKIP_CLEANUP ]; then
    echo "+ Cleanup..."
    docker_compose_cmd down -v --remove-orphans
fi

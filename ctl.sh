#!/bin/bash

container_name=lb
network_name=ebpf
image_name_tag=ubuntu-ebpf-lb:dev
docker_file=Dockerfile.lb
is_intel=$(uname -a | grep x86)
platform=linux/arm64/v8
if [ -n "${is_intel}" ]; then
  platform=linux/amd64
fi

has_container() {
    [ $( docker ps -a | grep $container_name | wc -l ) -gt 0 ]
}

start_container() {
    echo "Starting existing container"
    docker start ${container_name}
}

start_new_container() {
	docker network inspect ${network_name} >/dev/null 2>&1 || \
    docker network create --driver bridge ${network_name}
	docker run -it -d --network ${network_name} -v $(pwd)/lb:/code --platform=$platform --privileged --env TERM=xterm-color --name ${container_name} -h ${container_name} ${image_name_tag}
}

do_start_or_run()
{
    if has_container; then
	    start_container
    else
	    start_new_container
    fi
}

remove_container() {
    docker stop ${container_name} >& /dev/null
    docker rm ${container_name} >& /dev/null
}

run_shell() {
    docker exec -it $container_name bash
}

build_image() {
	docker build --platform=$platform -t ${image_name_tag} -f ${docker_file} .
}

remove_image() {
    docker rmi ${image_name_tag}
}

stop_container() {
    docker stop $container_name
}

do_help()
{
    cat <<EOF
Usage $0:  [command] [command opts...]

Commands:
 start        Start the container
 run          Docker run the container for the first time
 stop         Stop the container
 clean        Stop and remove the container
 shell        Remove container state and restart
 clean-image  Remove all container state and the image
 build        Build the container image locally

If no command is specified, the default is 'start'.
EOF
}

main()
{
    # Default subcommand
    if [[ $# == 0 ]]; then
	do_start_or_run
	exit 0
    fi

    # Subcommands
    case $1 in
	help)
	    do_help
	    exit 0
	    ;;
	start|run)
	    shift
	    do_start_or_run
	    ;;
	shell)
	    shift
	    run_shell
	    ;;
	stop)
	    shift
	    stop_container
	    ;;
	clean)
	    shift
	    remove_container
	    ;;
	build)
	    shift
	    remove_container
	    build_image
	    ;;
	clean-image)
	    remove_container
	    remove_image
	    ;;
	*)
	    echo "Invalid command $1"
	    do_help
	    exit 1
	    ;;
    esac
}

main $@

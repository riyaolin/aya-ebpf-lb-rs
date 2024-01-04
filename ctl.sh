#!/bin/bash

container_name=lb
network_name=ebpf
image_name_tag=ubuntu-ebpf-lb:dev

has_container() {
    [ $( docker ps -a | grep $container_name | wc -l ) -gt 0 ]
}

has_network() {
    local rv
    rv=0
    $(docker network inspect "${network_name}" 2>&1 > /dev/null) || rv=$?
    if [[ $rv == 0 ]]; then
	    return 0
    else
	    return 1
    fi
}

start_container() {
    echo "Starting existing container"
    docker start ${container_name}
}

start_new_container() {
	docker run -it -d --network ${network_name} -v $(pwd)/lb:/code --platform=linux/arm64/v8 --privileged --env TERM=xterm-color --name ${container_name} -h ${container_name} ${image_name_tag}
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
	docker build --platform=linux/arm64/v8 -t ${image_name_tag} -f Dockerfile.lb .
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

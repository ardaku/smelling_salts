#define _GNU_SOURCE
#include <sys/epoll.h>
#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include <fcntl.h>

int epolle_new(void) {
    int epoll_fd = epoll_create1(0 /* no flags */);
    if(epoll_fd < 0) {
        printf("Failed to create epoll instance");
        exit(1);
    }
    return epoll_fd;
}

void epolle_add(int epoll_fd, int pipe_in, void* ptr) {
    struct epoll_event ev;
    ev.events = EPOLLIN | EPOLLOUT;
    ev.data.ptr = ptr;
    int result = epoll_ctl(epoll_fd, EPOLL_CTL_ADD, pipe_in, &ev);
    if(result < 0) {
        printf("Failed to add pipe to epoll.");
        exit(1);
    }
}

int main(int argc, char* argv[]) {
    int* ptr = malloc(sizeof(int) * 16);
    // Create epoll instance
    int epoll_fd = epolle_new();
    // Create pipe
    int pipe[2];
    int result = pipe2(pipe, 0 /* no flags */);
    if(result < 0) {
        printf("Failed to create pipe");
        exit(1);
    }
    int pipe_in = pipe[0]; // read
    int pipe_out = pipe[1]; // write

    // Add pipe to epoll instance
    epolle_add(epoll_fd, pipe_in, ptr);

    // Write to pipe
    char data[1] = { 1 };
    result = write(pipe_out, data, 1);
    if(result < 0) {
        printf("Failed to write to pipe");
        exit(1);
    }

    // Wait for pipe message, then quit
    while(1) {
        struct epoll_event ev;

        epoll_wait(epoll_fd, &ev, 1 /* Get one event at a time */, -1 /* wait indefinitely */);

        if(ev.data.ptr == ptr) {
            printf("%p = %p\n", ev.data.ptr, ptr);
            break;
        } else {
            printf("%p ≠ %p\n", ev.data.ptr, ptr);
        }
    }

    return 0;
}

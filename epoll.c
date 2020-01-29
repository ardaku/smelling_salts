// #define _GNU_SOURCE
#include <sys/epoll.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include <fcntl.h>

static const uint32_t _WT_EPOLLIN = 0x001;
static const uint32_t _WT_EPOLLOUT = 0x004;

static const uint32_t _WT_EPOLL_CTL_ADD = 1;
static const uint32_t _WT_EPOLL_CTL_DEL = 2;
static const uint32_t _WT_EPOLL_CTL_MOD = 3;

typedef union _wt_epoll_data
{
  void *ptr;
  int fd;
  uint32_t u32;
  uint64_t u64;
} _wt_epoll_data_t;

struct _wt_epoll_event
{
  uint32_t events;      /* Epoll events */
  _wt_epoll_data_t data;    /* User data variable */
} __EPOLL_PACKED;

/*typedef union epoll_data {
    void    *ptr;
    int      fd;
    uint32_t u32;
    uint64_t u64;
} epoll_data_t;

struct epoll_event {
    uint32_t     events;
    epoll_data_t data;
};*/

int epoll_wait(int epfd, struct epoll_event *events, int maxevents,
    int timeout);
int epoll_create1(int flags);
int epoll_ctl(int epfd, int op, int fd, struct epoll_event *event);
int pipe2(int pipefd[2], int flags);

int epolle_new(void) {
    // printf("%X\n", O_CLOEXEC);
    int epoll_fd = epoll_create1(O_CLOEXEC /* no flags */);
    if(epoll_fd < 0) {
        printf("Failed to create epoll instance");
        exit(1);
    }
    return epoll_fd;
}

void epolle_add(int epoll_fd, int pipe_in, void* ptr) {
    struct epoll_event ev;
    ev.events = _WT_EPOLLIN | _WT_EPOLLOUT;
    ev.data.ptr = ptr;
    int result = epoll_ctl(epoll_fd, _WT_EPOLL_CTL_ADD, pipe_in, &ev);
    if(result < 0) {
        printf("Failed to add pipe to epoll.");
        exit(1);
    }
}

void pipe_new(int pipe[]) {
    int result = pipe2(pipe, 0 /* no flags */);
    if(result < 0) {
        printf("Failed to create pipe");
        exit(1);
    }
}

void pipe_write(int pipe_out) {
    char data[1] = { 1 };
    int result = write(pipe_out, data, 1);
    if(result < 0) {
        printf("Failed to write to pipe");
        exit(1);
    }
}

int epolle_wait(struct epoll_event* ev, int epoll_fd) {
    int result = epoll_wait(
        epoll_fd,
        ev,
        1 /* Get one event at a time */,
        -1 /* wait indefinitely */
    );

    if (result < 0) {
        return -1;
    }

    return 0;
}

int main(int argc, char* argv[]) {
    printf("%ld, %ld\n", sizeof(struct _wt_epoll_event), sizeof(struct epoll_event));

    int* ptr = (int*) (((uintptr_t)malloc(sizeof(int) * 16)) << 32);
    // Create epoll instance
    int epoll_fd = epolle_new();
    // Create pipe
    int pipe[2];
    pipe_new(pipe);
    int pipe_in = pipe[0]; // read
    int pipe_out = pipe[1]; // write
    // Add pipe to epoll instance
    epolle_add(epoll_fd, pipe_in, ptr);
    // Write to pipe
    pipe_write(pipe_out);

    // Wait for pipe message, then quit
    while(1) {
        struct epoll_event ev;
        if(epolle_wait(&ev, epoll_fd) < 0) {
            printf("Fail\n");
            continue;
        }

        if(ev.data.ptr == ptr) {
            printf("%p = %p\n", ev.data.ptr, ptr);
            break;
        } else {
            printf("%p ≠ %p\n", ev.data.ptr, ptr);
            break;
        }
    }

    return 0;
}

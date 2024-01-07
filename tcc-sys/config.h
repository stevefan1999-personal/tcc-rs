#ifdef CONFIG_VFS
#define open vfs_open
#define read vfs_read
#define lseek vfs_lseek
#define close vfs_close
#endif
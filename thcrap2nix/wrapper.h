#include <stddef.h>
#include <stdint.h>
//#include <stdbool.h>

typedef int BOOL;
typedef void *PVOID;
typedef PVOID HANDLE;
/// <div rustbindgen hide></div>
typedef void json_t;
typedef unsigned long DWORD;


#define bool	_Bool
//////////////////////


typedef struct
{
	char *repo_id;
	char *patch_id;
} patch_desc_t;

// Patch data from runconfig and patch.js
typedef struct
{
	// Patch root path
	// Pulled from the run configuration then edited to make it an absolute path
	char *archive;
	size_t archive_length;
	// Patch id (from patch.js)
	char *id;
	// Patch version (from patch.js)
	uint32_t version;
	// Patch description (from patch.js)
	char *title;
	// Servers list (NULL-terminated) (from patch.js)
	char **servers;
	// List of dependencies (from patch.js)
	patch_desc_t *dependencies;
	// List of font files to load (from patch.js)
	char **fonts;
	// List of files to ignore (NULL-terminated) (from run configuration)
	char **ignore;
	// If false, the updater should ignore this patch
	bool update;
	// Patch index in the stack, used for pretty-printing
	size_t level;
	// User set patch configuration
	json_t *config;

	// MOTD: message that can be displayed by a patch on startup (from patch.js)
	// Message content
	const char *motd;
	// Message title (optional)
	const char *motd_title;
	// Message type. See the uType parameter in https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-messagebox
	// Optional, defaults to 0 (MB_OK)
	DWORD motd_type;
} patch_t;

//////////////////////


typedef struct {
	char *patch_id;
	char *title;
} repo_patch_t;

typedef struct {
	char *id;
	char *title;
	char *contact;
	char **servers;
	char **neighbors;
	repo_patch_t *patches;
} repo_t;

// Returns the local file name of a repository file.
char *RepoGetLocalFN(const char *id);

repo_t *RepoLocalNext(HANDLE *hFind);

// Loads a repository from a repo.js.
//repo_t *RepoLoadJson(json_t *repo_js);

// Loads repository files from all subdirectories of the current directory.
repo_t **RepoLoad(void);

// Write the repo.js for repo to repos/[repo_name]/repo.js
bool RepoWrite(const repo_t *repo);

// Free a repo returned by RepoLoad, RepoLoadJson or RepoLocalNext.
void RepoFree(repo_t *repo);


typedef enum {
    // Download in progress. file_size may be 0 if it isn't known yet.
    GET_DOWNLOADING,
    // Download completed successfully
    GET_OK,
    // Error with the file (4XX error codes)
    GET_CLIENT_ERROR,
    // The downloaded file doesn't match the CRC32 in files.js
    GET_CRC32_ERROR,
    // Error with the server (timeout, 5XX error code etc)
    GET_SERVER_ERROR,
    // Download cancelled. You will see this if you return
    // false to the progress callback, or if we tried to
    // download a file from 2 different URLs at the same time.
    GET_CANCELLED,
    // Internal error in the download library or when
    // writing the file
    GET_SYSTEM_ERROR,
} get_status_t;

typedef struct {
    // Patch containing the file in download
    const patch_t *patch;
    // File name
    const char *fn;
    // Download URL
    const char *url;
    // File download status or result
    get_status_t status;
    // Human-readable error message if status is
    // GET_CLIENT_ERROR, GET_SERVER_ERROR or
    // GET_SYSTEM_ERROR, nullptr otherwise.
    const char *error;

    // Bytes downloaded for the current file
    size_t file_progress;
    // Size of the current file
    size_t file_size;

    // Number of files downloaded in the current session
    size_t nb_files_downloaded;
    // Number of files to download. Note that it will be 0 if
    // all the files.js haven't been downloaded yet.
    size_t nb_files_total;
} progress_callback_status_t;

typedef bool (*progress_callback_t)(progress_callback_status_t *status, void *param);

typedef int (*update_filter_func_t)(const char *fn, void *filter_data);


int update_filter_global_wrapper(const char *fn, void*);
int update_filter_games_wrapper(const char *fn, void *games);
void stack_update_wrapper(update_filter_func_t filter_func, void *filter_data, progress_callback_t progress_callback, void *progress_param);
BOOL loader_update_with_UI_wrapper(const char *exe_fn, char *args);

int update_notify_thcrap_wrapper();

repo_t ** RepoDiscover_wrapper(const char *start_url);
patch_t patch_bootstrap_wrapper(const patch_desc_t *sel, const repo_t *repo);

void thcrap_update_exit_wrapper();


///////////////////
patch_t patch_init(const char *patch_path, const json_t *patch_info, size_t level);

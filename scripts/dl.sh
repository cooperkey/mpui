#!/bin/bash
# ==============================================================================
# Media Grabber - Interactive TUI Video & Audio Downloader
# Powered by gum, yt-dlp, and ffmpeg
# ==============================================================================

set -euo pipefail

# --- Color & Style Helpers ---
BOLD="\033[1m"
GREEN="\033[32m"
YELLOW="\033[33m"
CYAN="\033[36m"
RED="\033[31m"
RESET="\033[0m"

# Ensure cleanup on exit
TMP_DIR="$(mktemp -d -t mediagrabber-XXXXXX 2>/dev/null || mktemp -d /tmp/mediagrabber-XXXXXX)"
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

# --- Dependency Verification ---
check_dependency() {
    local cmd="$1"
    local install_hint="$2"
    if ! command -v "$cmd" &>/dev/null; then
        echo -e "${RED}Error:${RESET} Required tool '${BOLD}$cmd${RESET}' is not installed or not in PATH."
        echo -e "Install hint: ${CYAN}$install_hint${RESET}"
        exit 1
    fi
}

check_dependency "gum" "brew install gum  OR  sudo port install gum"
check_dependency "yt-dlp" "brew install yt-dlp  OR  sudo port install yt-dlp"
check_dependency "ffmpeg" "brew install ffmpeg  OR  sudo port install ffmpeg"

# --- Banner Display ---
show_banner() {
    if command -v gum &>/dev/null; then
        gum style \
            --border rounded \
            --margin "1" \
            --padding "1 3" \
            --border-foreground 212 \
            --align center \
            --bold \
            "MEDIA GRABBER TUI" \
            "Universal Video & Audio Downloader" \
            "Powered by gum • yt-dlp • ffmpeg"
    else
        echo -e "${CYAN}=====================================================${RESET}"
        echo -e "${BOLD}MEDIA GRABBER TUI${RESET} - Video & Audio Downloader"
        echo -e "${CYAN}=====================================================${RESET}"
    fi
}

# --- Main Interactive Loop ---
main_loop() {
    local initial_url="${1:-}"
    local default_mode="${PRESET_MODE:-}"
    local dry_run="${DRY_RUN:-0}"

    while true; do
        clear 2>/dev/null || true
        show_banner

        # 1. URL Acquisition
        local url=""
        if [[ -n "$initial_url" ]]; then
            url="$initial_url"
            initial_url="" # Only use once
        else
            # Try reading macOS clipboard if available
            local clip_url=""
            if command -v pbpaste &>/dev/null; then
                local pb_content
                pb_content="$(pbpaste 2>/dev/null | tr -d '\r' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' || true)"
                if [[ "$pb_content" =~ ^https?:// ]]; then
                    clip_url="$pb_content"
                fi
            fi

            url=$(gum input \
                --placeholder "Paste video, audio, or playlist URL..." \
                --value "$clip_url" \
                --header "Paste URL (YouTube, Twitter/X, Reddit, TikTok, SoundCloud, etc.):" \
                --width 80)
        fi

        # Strip spaces and surrounding quotes
        url="$(echo "$url" | sed "s/^[ '\"\t]*//;s/[ '\"\t]*$//")"

        if [[ -z "$url" ]]; then
            echo -e "${YELLOW}No URL entered.${RESET}"
            if ! gum confirm "Exit Media Grabber?"; then
                continue
            else
                exit 0
            fi
        fi

        if [[ ! "$url" =~ ^https?:// ]]; then
            gum style --foreground 196 "[Error] Invalid URL. URL must start with http:// or https://"
            sleep 2
            continue
        fi

        # 2. Inspect Metadata with yt-dlp
        local meta_json="$TMP_DIR/meta.json"
        rm -f "$meta_json"

        gum spin --spinner dot --title "Analyzing link and fetching metadata..." -- \
            yt-dlp --dump-single-json --flat-playlist --no-warnings "$url" > "$meta_json" 2>/dev/null || true

        if [[ ! -s "$meta_json" ]]; then
            gum style --foreground 196 "[Error] Failed to extract media information from this URL. Please verify the link."
            if gum confirm "Would you like to try another URL?"; then
                continue
            else
                exit 1
            fi
        fi

        # Parse metadata via Python standard library
        local parsed_meta
        parsed_meta=$(python3 - "$meta_json" << 'EOF'
import json, sys

try:
    with open(sys.argv[1], 'r', encoding='utf-8') as f:
        d = json.load(f)
except Exception:
    sys.exit(1)

is_playlist = False
item_type = d.get('_type', '')
entries = d.get('entries')

if item_type == 'playlist' or (entries is not None and isinstance(entries, list)):
    is_playlist = True

title = str(d.get('title') or 'Unknown Title').replace('\n', ' ').replace('\r', ' ').strip()
uploader = str(d.get('uploader') or d.get('channel') or d.get('uploader_id') or 'Unknown Uploader').replace('\n', ' ').replace('\r', ' ').strip()
extractor = str(d.get('extractor_key') or d.get('extractor') or 'Web').replace('\n', ' ').strip()
playlist_count = d.get('playlist_count')
if playlist_count is None and entries:
    playlist_count = len(entries)

duration = d.get('duration_string') or ''
if not duration and d.get('duration'):
    secs = int(d['duration'])
    mins, s = divmod(secs, 60)
    h, m = divmod(mins, 60)
    duration = f"{h:d}:{m:02d}:{s:02d}" if h else f"{m:d}:{s:02d}"

print(f"IS_PLAYLIST={1 if is_playlist else 0}")
print(f"TITLE={title}")
print(f"UPLOADER={uploader}")
print(f"EXTRACTOR={extractor}")
print(f"COUNT={playlist_count if playlist_count else 0}")
print(f"DURATION={duration}")
print(f"PLAYLIST_ID={d.get('id', 'archive')}")
EOF
)

        local is_playlist=0
        local media_title="Unknown"
        local media_uploader="Unknown"
        local media_extractor="Web"
        local playlist_count=0
        local media_duration=""
        local playlist_id="archive"

        while IFS='=' read -r key val; do
            case "$key" in
                IS_PLAYLIST) is_playlist="$val" ;;
                TITLE) media_title="$val" ;;
                UPLOADER) media_uploader="$val" ;;
                EXTRACTOR) media_extractor="$val" ;;
                COUNT) playlist_count="$val" ;;
                DURATION) media_duration="$val" ;;
                PLAYLIST_ID) playlist_id="$val" ;;
            esac
        done <<< "$parsed_meta"

        # Also detect if URL has playlist parameter even if flat-playlist extracted first entry
        if [[ "$url" == *"list="* || "$url" == *"/playlist"* ]]; then
            is_playlist=1
        fi

        # Show Media Info Card
        local info_box
        if [[ "$is_playlist" -eq 1 ]]; then
            info_box=$(gum style \
                --border normal \
                --padding "0 2" \
                --border-foreground 39 \
                "Type:        Playlist ($media_extractor)" \
                "Title:       $media_title" \
                "Creator:     $media_uploader" \
                "Videos:      ${playlist_count:-Multiple} items")
        else
            info_box=$(gum style \
                --border normal \
                --padding "0 2" \
                --border-foreground 39 \
                "Type:        Single Video / Audio ($media_extractor)" \
                "Title:       $media_title" \
                "Creator:     $media_uploader" \
                "Duration:    ${media_duration:-Unknown}")
        fi
        echo "$info_box"
        echo ""

        # Array for yt-dlp command arguments
        local -a ytdl_args=()
        ytdl_args+=("--continue" "--concurrent-fragments" "5")

        # 3. Playlist Options (if playlist)
        local playlist_folder_prefix=""
        if [[ "$is_playlist" -eq 1 ]]; then
            local pl_choice
            pl_choice=$(gum choose \
                --header "Playlist Detected. How would you like to proceed?" \
                "Download Entire Playlist (${playlist_count:-all} items)" \
                "Download Specific Range / Items (e.g., 1-10, 5, 20:)" \
                "Download Single Video Only (Current link)")

            case "$pl_choice" in
                "Download Entire Playlist"*)
                    ytdl_args+=("--yes-playlist")
                    playlist_folder_prefix="%(playlist_title,playlist)s/"
                    ;;
                "Download Specific Range"*)
                    local range_input
                    range_input=$(gum input \
                        --placeholder "e.g. 1-10 or 1,3,5 or 10:" \
                        --header "Enter playlist range or item numbers:" \
                        --width 50)
                    if [[ -n "$range_input" ]]; then
                        ytdl_args+=("--playlist-items" "$range_input")
                    fi
                    playlist_folder_prefix="%(playlist_title,playlist)s/"
                    ;;
                "Download Single Video Only"*)
                    ytdl_args+=("--no-playlist")
                    is_playlist=0
                    ;;
            esac

            # For playlists: always ignore broken videos & use resume archive
            if [[ "$is_playlist" -eq 1 ]]; then
                ytdl_args+=("--ignore-errors" "--no-abort-on-error")
            fi
        else
            ytdl_args+=("--no-playlist")
        fi

        # 4. Download Mode Selection
        local dl_mode="$default_mode"
        if [[ -z "$dl_mode" ]]; then
            dl_mode=$(gum choose \
                --header "Select Download Mode:" \
                "Audio Only (MP3, FLAC, M4A, OPUS)" \
                "Video + Audio (MP4, MKV, WebM)" \
                "Fast Best (Quickest maximum quality video)" \
                "Custom Format Code (Pick from yt-dlp -F)")
        fi

        local mode_summary=""

        case "$dl_mode" in
            *"Audio Only"*|"audio")
                local audio_fmt
                audio_fmt=$(gum choose \
                    --header "Choose Audio Format:" \
                    --selected="mp3" \
                    "mp3   - Universal compatibility (ID3 tags)" \
                    "m4a   - AAC, excellent quality (Apple / iOS)" \
                    "flac  - Lossless audiophile quality" \
                    "opus  - High fidelity, state-of-the-art compression" \
                    "best  - Native audio stream (no re-encoding)")

                local clean_fmt
                clean_fmt=$(echo "$audio_fmt" | awk '{print $1}')

                local audio_qual="320k"
                if [[ "$clean_fmt" != "best" ]]; then
                    local qual_choice
                    qual_choice=$(gum choose \
                        --header "Choose Audio Bitrate / Quality:" \
                        --selected="320k (Best Quality CBR)" \
                        "320k (Best Quality CBR)" \
                        "0    (Best Quality VBR)" \
                        "256k (High Quality)" \
                        "192k (Standard Quality)" \
                        "128k (Compact Size)")
                    audio_qual=$(echo "$qual_choice" | awk '{print $1}')
                fi

                ytdl_args+=("-x")
                if [[ "$clean_fmt" != "best" ]]; then
                    ytdl_args+=("--audio-format" "$clean_fmt" "--audio-quality" "$audio_qual")
                fi

                # Embed metadata and album art thumbnail (converted to jpg for maximum compatibility)
                ytdl_args+=("--embed-metadata" "--embed-thumbnail" "--convert-thumbnails" "jpg")
                mode_summary="Audio ($clean_fmt @ ${audio_qual})"
                ;;

            *"Video + Audio"*|"video")
                local res_choice
                res_choice=$(gum choose \
                    --header "Choose Video Resolution:" \
                    --selected="1080p (Full HD)" \
                    "best  - Highest Available (4K / 2K / 1080p)" \
                    "1080p - Full HD (1080p)" \
                    "720p  - HD (720p)" \
                    "480p  - SD (480p / Data saver)")

                case "$res_choice" in
                    "best"*)
                        ytdl_args+=("-f" "bv*+ba/b")
                        ;;
                    "1080p"*)
                        ytdl_args+=("-f" "bv*[height<=1080]+ba/b[height<=1080]/bv*+ba/b")
                        ;;
                    "720p"*)
                        ytdl_args+=("-f" "bv*[height<=720]+ba/b[height<=720]/bv*+ba/b")
                        ;;
                    "480p"*)
                        ytdl_args+=("-f" "bv*[height<=480]+ba/b[height<=480]/bv*+ba/b")
                        ;;
                esac

                local container_choice
                container_choice=$(gum choose \
                    --header "Choose Video Container Format:" \
                    --selected="mp4" \
                    "mp4   - Universal compatibility" \
                    "mkv   - Recommended for subtitles & multi-audio" \
                    "webm  - Modern open web standard" \
                    "auto  - Keep default source format")

                local clean_container
                clean_container=$(echo "$container_choice" | awk '{print $1}')
                if [[ "$clean_container" != "auto" ]]; then
                    ytdl_args+=("--merge-output-format" "$clean_container")
                fi

                # Subtitles
                local sub_choice
                sub_choice=$(gum choose \
                    --header "Subtitles:" \
                    --selected="None" \
                    "None" \
                    "Embed English Subtitles (auto & manual)" \
                    "Embed All Available Subtitles" \
                    "Save English Subtitles as external .srt file" \
                    "Save All Subtitles as external .srt files")

                case "$sub_choice" in
                    "Embed English"*)
                        ytdl_args+=("--embed-subs" "--sub-langs" "en.*,en" "--write-auto-subs")
                        ;;
                    "Embed All"*)
                        ytdl_args+=("--embed-subs" "--sub-langs" "all" "--write-auto-subs")
                        ;;
                    "Save English"*)
                        ytdl_args+=("--write-subs" "--write-auto-subs" "--sub-langs" "en.*,en" "--convert-subs" "srt")
                        ;;
                    "Save All"*)
                        ytdl_args+=("--write-subs" "--write-auto-subs" "--sub-langs" "all" "--convert-subs" "srt")
                        ;;
                esac

                # Extras
                ytdl_args+=("--embed-chapters" "--embed-metadata" "--embed-thumbnail")
                mode_summary="Video (${res_choice%% -*}, ${clean_container})"
                ;;

            *"Fast Best"*)
                ytdl_args+=("-f" "bv*+ba/b" "--merge-output-format" "mp4" "--embed-metadata" "--embed-thumbnail")
                mode_summary="Fast Best (Max Quality MP4)"
                ;;

            *"Custom Format Code"*)
                echo -e "${CYAN}Available formats for this media:${RESET}"
                yt-dlp -F "$url"
                echo ""
                local custom_f
                custom_f=$(gum input \
                    --placeholder "e.g. 137+140 or 22 or b" \
                    --header "Enter the format code(s) to download:")
                if [[ -n "$custom_f" ]]; then
                    ytdl_args+=("-f" "$custom_f")
                    mode_summary="Custom Format ($custom_f)"
                else
                    ytdl_args+=("-f" "bv*+ba/b")
                    mode_summary="Fast Best (fallback)"
                fi
                ;;
        esac

        # 5. Choose Destination Directory (Default: ~/Downloads)
        local dest_choice
        dest_choice=$(gum choose \
            --header "Select Destination Folder:" \
            --selected="~/Downloads" \
            "~/Downloads" \
            "~/Music" \
            "Current Directory (.)" \
            "~/Movies" \
            "Custom path...")

        local dest_dir=""
        case "$dest_choice" in
            "~/Downloads")
                dest_dir="$HOME/Downloads"
                ;;
            "~/Music")
                dest_dir="$HOME/Music"
                ;;
            "Current Directory (.)")
                dest_dir="$(pwd)"
                ;;
            "~/Movies")
                dest_dir="$HOME/Movies"
                ;;
            "Custom path...")
                dest_dir=$(gum input \
                    --placeholder "/path/to/download/directory" \
                    --value "$HOME/Downloads" \
                    --header "Enter destination directory path:")
                ;;
        esac

        # Expand tilde if user typed it
        dest_dir="${dest_dir/#\~/$HOME}"
        mkdir -p "$dest_dir"

        # Output template & playlist archive
        if [[ "$is_playlist" -eq 1 ]]; then
            local clean_pl_id
            clean_pl_id=$(echo "$playlist_id" | tr -cd '[:alnum:]_-')
            local archive_file="${dest_dir}/.archive_${clean_pl_id}.txt"
            ytdl_args+=("--download-archive" "$archive_file")
            ytdl_args+=("-o" "${dest_dir}/${playlist_folder_prefix}%(playlist_index)03d - %(title)s.%(ext)s")
        else
            ytdl_args+=("-o" "${dest_dir}/%(title)s.%(ext)s")
        fi

        # 6. Summary & Confirmation
        echo ""
        local summary_card
        summary_card=$(gum style \
            --border double \
            --padding "1 2" \
            --border-foreground 212 \
            "READY TO DOWNLOAD" \
            "" \
            "Target:      $media_title" \
            "Mode:        $mode_summary" \
            "Destination: $dest_dir" \
            "Source:      $media_extractor")
        echo "$summary_card"
        echo ""

        if [[ "$dry_run" == "1" ]]; then
            echo -e "${YELLOW}[DRY RUN MODE] Command that would execute:${RESET}"
            echo -e "${CYAN}yt-dlp ${ytdl_args[*]} \"$url\"${RESET}"
            exit 0
        fi

        if ! gum confirm "Start download now?"; then
            echo -e "${YELLOW}Download cancelled.${RESET}"
            if gum confirm "Would you like to start over with a new link?"; then
                continue
            else
                exit 0
            fi
        fi

        # 7. Execute Download
        echo ""
        echo -e "${BOLD}${GREEN}Downloading...${RESET}"
        echo ""

        local exit_code=0
        yt-dlp "${ytdl_args[@]}" "$url" || exit_code=$?

        echo ""
        if [[ $exit_code -eq 0 ]]; then
            # macOS notification
            if command -v osascript &>/dev/null; then
                local safe_title
                safe_title=$(echo "$media_title" | tr '"' "'" | head -c 50)
                osascript -e "display notification \"Finished: ${safe_title}\" with title \"Media Grabber\"" 2>/dev/null || true
            fi

            gum style \
                --border normal \
                --padding "1 2" \
                --border-foreground 82 \
                --bold \
                "Download Finished Successfully!" \
                "Saved in: $dest_dir"
        else
            gum style \
                --border normal \
                --padding "1 2" \
                --border-foreground 196 \
                --bold \
                "Download finished with code $exit_code." \
                "(If some playlist items failed or were blocked, they were skipped safely)"
        fi

        echo ""
        # 8. Post-Download Actions
        local next_step
        next_step=$(gum choose \
            --header "What would you like to do next?" \
            "Open destination folder in Finder" \
            "Download another link" \
            "Exit")

        case "$next_step" in
            "Open destination folder in Finder")
                if command -v open &>/dev/null; then
                    open "$dest_dir"
                fi
                if gum confirm "Download another link?"; then
                    continue
                else
                    break
                fi
                ;;
            "Download another link")
                continue
                ;;
            "Exit")
                break
                ;;
        esac
    done

    echo -e "${CYAN}Thanks for using Media Grabber!${RESET}"
}

# --- Entry Point CLI Parsing ---
TARGET_URL=""
PRESET_MODE=""
DRY_RUN=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)
            show_banner
            echo -e "${BOLD}Usage:${RESET}"
            echo "  ./dl.sh                      # Launch interactive TUI"
            echo "  ./dl.sh <URL>                # Launch interactive TUI pre-loaded with URL"
            echo "  ./dl.sh -a, --audio <URL>    # Direct audio download mode"
            echo "  ./dl.sh -v, --video <URL>    # Direct video download mode"
            echo "  ./dl.sh --dry-run <URL>      # Simulate and display generated yt-dlp command"
            echo "  ./dl.sh --help               # Show this help message"
            echo ""
            echo -e "${BOLD}Features:${RESET}"
            echo "  - Paste any link (YouTube, Twitter/X, TikTok, Reddit, SoundCloud, etc.)"
            echo "  - Auto-reads URL from macOS clipboard if present"
            echo "  - Defaults to ~/Downloads for clean organization"
            echo "  - Support for long playlists with range selection & resume archive"
            echo "  - Audio extraction: MP3 (320k/V0), FLAC, M4A, OPUS with embedded cover art"
            echo "  - Video resolution presets (4K/1080p/720p), container choice, subtitles"
            echo "  - macOS Finder integration & completion notifications"
            exit 0
            ;;
        -a|--audio)
            PRESET_MODE="audio"
            shift
            ;;
        -v|--video)
            PRESET_MODE="video"
            shift
            ;;
        --dry-run)
            DRY_RUN=1
            shift
            ;;
        http://*|https://*)
            TARGET_URL="$1"
            shift
            ;;
        *)
            TARGET_URL="$1"
            shift
            ;;
    esac
done

export PRESET_MODE DRY_RUN
main_loop "$TARGET_URL"

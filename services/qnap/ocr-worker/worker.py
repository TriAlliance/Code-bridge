#!/usr/bin/env python3
"""
OCR Worker for Coachly Code Bridge

Watches a directory for new screenshots and extracts text using Tesseract OCR.
Outputs JSON files with OCR results alongside the original images.
"""

import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path

import pytesseract
from PIL import Image
from watchdog.events import FileSystemEventHandler
from watchdog.observers import Observer


WATCH_DIR = os.environ.get("WATCH_DIR", "/screenshots")
OUTPUT_DIR = os.environ.get("OUTPUT_DIR", "/ocr-output")
TESSERACT_LANG = os.environ.get("TESSERACT_LANG", "eng")

# Supported image extensions
IMAGE_EXTENSIONS = {".png", ".jpg", ".jpeg", ".gif", ".webp", ".avif", ".bmp"}


class OCRHandler(FileSystemEventHandler):
    """Handle file system events and process images."""

    def __init__(self, output_dir: str):
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(parents=True, exist_ok=True)
        self.processed = set()

    def on_created(self, event):
        if event.is_directory:
            return
        self.process_file(event.src_path)

    def on_modified(self, event):
        if event.is_directory:
            return
        self.process_file(event.src_path)

    def process_file(self, file_path: str):
        """Process a single image file."""
        path = Path(file_path)

        # Check if it's an image
        if path.suffix.lower() not in IMAGE_EXTENSIONS:
            return

        # Skip if already processed recently
        if file_path in self.processed:
            return

        # Wait for file to be fully written
        time.sleep(0.5)

        try:
            self.extract_text(path)
            self.processed.add(file_path)

            # Clean up old processed entries
            if len(self.processed) > 1000:
                self.processed = set(list(self.processed)[-500:])

        except Exception as e:
            print(f"Error processing {path}: {e}", file=sys.stderr)

    def extract_text(self, image_path: Path):
        """Extract text from an image using Tesseract OCR."""
        print(f"Processing: {image_path}")

        # Open and process image
        image = Image.open(image_path)

        # Extract text
        text = pytesseract.image_to_string(image, lang=TESSERACT_LANG)

        # Get image info
        width, height = image.size

        # Create output filename
        relative_path = image_path.relative_to(WATCH_DIR) if WATCH_DIR in str(image_path) else image_path.name
        output_path = self.output_dir / f"{relative_path}.json"
        output_path.parent.mkdir(parents=True, exist_ok=True)

        # Create result object
        result = {
            "source": str(image_path),
            "filename": image_path.name,
            "processed_at": datetime.utcnow().isoformat(),
            "dimensions": {
                "width": width,
                "height": height
            },
            "text": text.strip(),
            "word_count": len(text.split()),
            "char_count": len(text),
            "language": TESSERACT_LANG
        }

        # Write result
        with open(output_path, "w") as f:
            json.dump(result, f, indent=2)

        print(f"  → Extracted {result['word_count']} words → {output_path}")

        # Also write plain text file
        txt_path = self.output_dir / f"{relative_path}.txt"
        with open(txt_path, "w") as f:
            f.write(text)


def process_existing_files(handler: OCRHandler, watch_dir: str):
    """Process any existing files in the watch directory."""
    print(f"Scanning existing files in {watch_dir}...")

    count = 0
    for root, dirs, files in os.walk(watch_dir):
        for file in files:
            file_path = Path(root) / file
            if file_path.suffix.lower() in IMAGE_EXTENSIONS:
                # Check if OCR result already exists
                relative = file_path.relative_to(watch_dir)
                ocr_path = handler.output_dir / f"{relative}.json"

                if not ocr_path.exists():
                    handler.process_file(str(file_path))
                    count += 1

    print(f"Processed {count} existing files")


def main():
    print("=" * 50)
    print("Coachly Code Bridge - OCR Worker")
    print("=" * 50)
    print(f"Watch directory: {WATCH_DIR}")
    print(f"Output directory: {OUTPUT_DIR}")
    print(f"Tesseract language: {TESSERACT_LANG}")
    print("=" * 50)

    # Verify Tesseract is installed
    try:
        version = pytesseract.get_tesseract_version()
        print(f"Tesseract version: {version}")
    except Exception as e:
        print(f"Error: Tesseract not found: {e}", file=sys.stderr)
        sys.exit(1)

    # Create handler
    handler = OCRHandler(OUTPUT_DIR)

    # Process existing files
    process_existing_files(handler, WATCH_DIR)

    # Set up file watcher
    observer = Observer()
    observer.schedule(handler, WATCH_DIR, recursive=True)
    observer.start()

    print(f"\nWatching for new screenshots...")

    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        observer.stop()

    observer.join()


if __name__ == "__main__":
    main()

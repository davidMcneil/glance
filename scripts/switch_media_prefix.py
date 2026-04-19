#!/usr/bin/env python3

import argparse
import sqlite3
import sys

VALID_NAMES = {"luke", "adam"}

def main():
    parser = argparse.ArgumentParser(
        description="Switch /media/<name>/ prefix in SQLite DB (media + label tables)."
    )
    parser.add_argument("db", help="Path to SQLite .db file")
    parser.add_argument("name", help="Target name: 'adam' or 'luke'")

    args = parser.parse_args()
    target = args.name.lower()

    if target not in VALID_NAMES:
        print("Error: name must be 'adam' or 'luke'")
        sys.exit(1)

    # Determine prefixes
    old = "/media/luke/" if target == "adam" else "/media/adam/"
    new = f"/media/{target}/"

    conn = sqlite3.connect(args.db)
    cursor = conn.cursor()

    try:
        # Wrap in transaction
        conn.execute("BEGIN")

        # Update media table
        cursor.execute("""
            UPDATE media
            SET filepath = REPLACE(filepath, ?, ?)
            WHERE filepath LIKE ?
        """, (old, new, f"{old}%"))

        media_changes = conn.total_changes

        # Update label table
        cursor.execute("""
            UPDATE label
            SET filepath = REPLACE(filepath, ?, ?)
            WHERE filepath LIKE ?
        """, (old, new, f"{old}%"))

        label_changes = conn.total_changes - media_changes

        conn.commit()

        print(f"Updated media rows: {media_changes}")
        print(f"Updated label rows: {label_changes}")

    except Exception as e:
        conn.rollback()
        print("Error occurred, rolled back transaction:")
        print(e)
        sys.exit(1)

    finally:
        conn.close()


if __name__ == "__main__":
    main()
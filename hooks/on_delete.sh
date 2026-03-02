#!/usr/bin/env bash
# PORT: on_delete hook
# Called after a task is deleted.
# $1 = task id, $2 = task text
#
# Examples:
#   Archive deleted tasks to a separate file
#   Log deletions for auditing

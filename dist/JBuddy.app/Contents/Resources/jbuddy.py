# Smart JBuddy - Menu Bar App for macOS (Python)
# Keeps Teams "Available" by moving mouse slightly when idle > 240s

import rumps
import time
import subprocess
import os
import json
from threading import Thread
from datetime import datetime, date, timedelta
from Quartz.CoreGraphics import (
    CGEventSourceSecondsSinceLastEventType,
    kCGEventSourceStateHIDSystemState,
    kCGAnyInputEventType,
    CGEventCreate,
    CGEventGetLocation,
    CGWarpMouseCursorPosition,
    CGPoint,
    CGEventCreateMouseEvent,
    kCGEventMouseMoved,
    kCGMouseButtonLeft,
    CGEventPost,
    kCGHIDEventTap
)

# Core settings
CHECK_INTERVAL = 60
INACTIVITY_THRESHOLD = 240
WORK_HOURS_START = 9
WORK_HOURS_END = 18
MOUSE_MOVEMENT_INTERVAL = 60  # seconds between mouse movements
MOVEMENT_DISTANCE = 1  # pixels to move mouse

# Break reminder settings
BREAK_REMINDER_INTERVAL = 60 * 60  # 1 hour in seconds
BREAK_DURATION = 5 * 60  # 5 minutes in seconds

# Icons paths
ACTIVE_ICON_PATH = "/Users/user/Documents/Jbuddy/icons/active_icon/jbuddy_active.icns"
IDLE_ICON_PATH = "/Users/user/Documents/Jbuddy/icons/idle_icon/jbuddy_idle.icns"

# Stats file path
STATS_FILE_PATH = os.path.expanduser("~/Library/Application Support/JBuddy/stats.json")

# Create directory if it doesn't exist
os.makedirs(os.path.dirname(STATS_FILE_PATH), exist_ok=True)

def get_idle_time():
    idle = CGEventSourceSecondsSinceLastEventType(
        kCGEventSourceStateHIDSystemState,
        kCGAnyInputEventType
    )
    return idle

def is_working_hours():
    now = datetime.now()
    return now.weekday() < 5 and WORK_HOURS_START <= now.hour < WORK_HOURS_END

def move_mouse_slightly():
    event = CGEventCreate(None)
    cursor_pos = CGEventGetLocation(event)
    moves = [
        CGPoint(cursor_pos.x + MOVEMENT_DISTANCE, cursor_pos.y),
        CGPoint(cursor_pos.x + MOVEMENT_DISTANCE, cursor_pos.y + MOVEMENT_DISTANCE),
        CGPoint(cursor_pos.x, cursor_pos.y + MOVEMENT_DISTANCE),
        CGPoint(cursor_pos.x, cursor_pos.y)
    ]
    for pos in moves:
        CGWarpMouseCursorPosition(pos)
        # Generate a synthetic mouse move event to reset idle timer
        mouse_event = CGEventCreateMouseEvent(
            None,
            kCGEventMouseMoved,
            pos,
            kCGMouseButtonLeft
        )
        CGEventPost(kCGHIDEventTap, mouse_event)
        time.sleep(0.1)

class JBuddyApp(rumps.App):
    def __init__(self):
        super().__init__("JBuddy", icon=IDLE_ICON_PATH, quit_button=None)
        self.status_item = rumps.MenuItem("Status: Idle")
        self.is_manually_active = False
        self.toggle_item = rumps.MenuItem("Set Active Manually", callback=self.toggle_active_state)
        
        # Activity statistics menu item
        self.stats_item = rumps.MenuItem("Activity Statistics", callback=self.show_statistics)
        
        # Break reminder components
        self.last_break_time = datetime.now()
        self.break_reminder_item = rumps.MenuItem("Break Reminder: On", callback=self.toggle_break_reminder)
        self.break_active = False
        self.break_reminder_enabled = True
        
        # Main menu setup
        self.menu = [
            self.status_item, 
            self.toggle_item, 
            None,
            self.stats_item,
            self.break_reminder_item,
            None, 
            "Quit"
        ]
        
        # App state
        self.keep_running = True
        self.is_after_hours = False
        self.last_active_date = date.today()
        
        # Statistics tracking
        self.today_date = date.today()
        self.active_time = 0  # seconds
        self.idle_time = 0    # seconds
        self.last_state_time = datetime.now()
        self.is_active = False
        
        # Load existing statistics
        self.stats = self.load_statistics()
        
        # Start monitoring
        self.worker_thread = Thread(target=self.monitor_loop, daemon=True)
        self.worker_thread.start()

    def toggle_active_state(self, sender):
        self.is_manually_active = not self.is_manually_active
        if self.is_manually_active:
            sender.title = "Set Idle Manually"
            self.status_item.title = "Status: Active (Manual)"
            self.icon = ACTIVE_ICON_PATH
            self.update_active_state(True)
        else:
            sender.title = "Set Active Manually"
            self.status_item.title = "Status: Idle"
            self.icon = IDLE_ICON_PATH
            self.update_active_state(False)

    def toggle_break_reminder(self, sender):
        self.break_reminder_enabled = not self.break_reminder_enabled
        if self.break_reminder_enabled:
            sender.title = "Break Reminder: On"
        else:
            sender.title = "Break Reminder: Off"

    def load_statistics(self):
        if os.path.exists(STATS_FILE_PATH):
            try:
                with open(STATS_FILE_PATH, 'r') as f:
                    return json.load(f)
            except (json.JSONDecodeError, IOError) as e:
                return {"days": {}}
        return {"days": {}}

    def save_statistics(self):
        try:
            with open(STATS_FILE_PATH, 'w') as f:
                json.dump(self.stats, f)
        except IOError:
            pass

    def update_active_state(self, is_active):
        now = datetime.now()
        elapsed = (now - self.last_state_time).total_seconds()
        
        # Update the appropriate counter
        if self.is_active:
            self.active_time += elapsed
        else:
            self.idle_time += elapsed
            
        # Update current state
        self.is_active = is_active
        self.last_state_time = now
        
        # Check if day changed
        today = date.today()
        if today != self.today_date:
            # Save yesterday's stats
            self.save_daily_stats()
            # Reset for new day
            self.today_date = today
            self.active_time = 0
            self.idle_time = 0

    def save_daily_stats(self):
        # Convert seconds to hours
        active_hours = round(self.active_time / 3600, 2)
        idle_hours = round(self.idle_time / 3600, 2)
        
        # Format date as string (YYYY-MM-DD)
        date_str = self.today_date.isoformat()
        
        # Update stats dict
        self.stats["days"][date_str] = {
            "active_hours": active_hours,
            "idle_hours": idle_hours
        }
        
        # Save to file
        self.save_statistics()

    def show_statistics(self, _):
        # Save current stats first
        self.update_active_state(self.is_active)  # Update with current state before saving
        self.save_daily_stats()
        
        # Prepare stats message
        today_str = self.today_date.isoformat()
        today_stats = self.stats["days"].get(today_str, {"active_hours": 0, "idle_hours": 0})
        
        active_hours = today_stats["active_hours"]
        idle_hours = today_stats["idle_hours"]
        
        # Calculate for this week
        week_active = 0
        week_idle = 0
        
        # Get dates for this week (Monday to today)
        today = date.today()
        monday = today - timedelta(days=today.weekday())
        
        for i in range(7):  # Monday through Sunday
            day = monday + timedelta(days=i)
            if day > today:  # Don't include future days
                break
                
            day_str = day.isoformat()
            day_stats = self.stats["days"].get(day_str, {"active_hours": 0, "idle_hours": 0})
            week_active += day_stats["active_hours"]
            week_idle += day_stats["idle_hours"]
        
        # Calculate for this month
        month_active = 0
        month_idle = 0
        
        # Get dates for this month (1st to today)
        first_day = today.replace(day=1)
        
        # Loop through all days of the current month up to today
        current_day = first_day
        while current_day <= today:
            day_str = current_day.isoformat()
            day_stats = self.stats["days"].get(day_str, {"active_hours": 0, "idle_hours": 0})
            month_active += day_stats["active_hours"]
            month_idle += day_stats["idle_hours"]
            current_day += timedelta(days=1)
        
        message = f"Today's Activity:\n• Active: {active_hours:.1f} hours\n• Idle: {idle_hours:.1f} hours\n\nThis Week:\n• Active: {week_active:.1f} hours\n• Idle: {week_idle:.1f} hours\n\nThis Month:\n• Active: {month_active:.1f} hours\n• Idle: {month_idle:.1f} hours"
        rumps.alert("JBuddy Activity Statistics", message)

    def check_break_reminder(self):
        if not self.break_reminder_enabled or self.break_active:
            return
            
        now = datetime.now()
        time_since_break = (now - self.last_break_time).total_seconds()
        
        if time_since_break > BREAK_REMINDER_INTERVAL and self.is_active:
            # Time for a break!
            self.break_active = True
            self.show_break_reminder()

    def show_break_reminder(self):
        response = rumps.alert(
            "Time for a Break!",
            "You've been active for over an hour. Take a short break to stretch and rest your eyes.",
            "Start Break",
            "Skip"
        )
        
        if response == 1:  # User clicked "Start Break"
            self.status_item.title = "Status: On Break"
            rumps.notification(
                title="JBuddy Break Time",
                subtitle="Taking a 5-minute break",
                message="Your status will remain active during this time."
            )
            
            # Reset break timer
            self.last_break_time = datetime.now()
            self.break_active = False
        else:
            # User clicked "Skip", remind again in 10 minutes
            self.last_break_time = datetime.now() - timedelta(seconds=BREAK_REMINDER_INTERVAL - 600)
            self.break_active = False

    def monitor_loop(self):
        while self.keep_running:
            now = datetime.now()
            today = date.today()
            
            # Check for break reminder
            self.check_break_reminder()
            
            # Check if we've entered a new day during working hours
            if self.is_after_hours and today > self.last_active_date and now.hour >= WORK_HOURS_START:
                self.is_after_hours = False
                self.status_item.title = "Status: Active (New Day)"
                self.icon = ACTIVE_ICON_PATH
                self.update_active_state(True)
            
            # During work hours
            if not self.is_after_hours and is_working_hours():
                self.last_active_date = today
                
                if self.is_manually_active:
                    self.status_item.title = "Status: Active (Manual)"
                    self.icon = ACTIVE_ICON_PATH
                    move_mouse_slightly()
                    self.update_active_state(True)
                else:
                    idle = get_idle_time()
                    if idle > INACTIVITY_THRESHOLD:
                        self.status_item.title = "Status: Active (Auto)"
                        self.icon = ACTIVE_ICON_PATH
                        move_mouse_slightly()
                        self.update_active_state(True)
                    else:
                        self.status_item.title = "Status: Idle"
                        self.icon = IDLE_ICON_PATH
                        self.update_active_state(False)
            # After work hours
            elif not self.is_after_hours and now.hour >= WORK_HOURS_END:
                self.is_after_hours = True
                self.status_item.title = "Status: After Hours"
                self.icon = IDLE_ICON_PATH
                self.update_active_state(False)
            
            # Save stats periodically
            if now.minute % 5 == 0 and now.second < 5:  # Every 5 minutes
                self.save_daily_stats()
                
            time.sleep(MOUSE_MOVEMENT_INTERVAL)

    @rumps.clicked("Quit")
    def quit_app(self, _):
        # Save stats before quitting
        self.update_active_state(self.is_active)  # Update with current state before saving
        self.save_daily_stats()
        self.keep_running = False
        rumps.quit_application()

if __name__ == "__main__":
    JBuddyApp().run()
# JBuddy

JBuddy is a macOS menu bar application that helps keep Microsoft Teams active during working hours by simulating user activity, preventing your status from going idle.

![JBuddy Icons](icons/active_icon/jbuddy_active.png)

## Features

- **Activity Simulation**: Keeps Teams active by simulating minimal keyboard activity
- **Working Hours**: Set your own working hours for when JBuddy should keep you active
- **Menu Bar Access**: Quick access to enable/disable through the menu bar
- **Automatic Scheduling**: Activates only during specified working hours
- **Activity Statistics**: Tracks usage and provides insights
- **Break Reminders**: Option to enable periodic reminders to take breaks

## Why JBuddy?

JBuddy was designed for remote workers who need to maintain their "Available" status in Microsoft Teams, even during short periods of inactivity. It helps prevent the following issues:

- Teams status automatically changing to "Away" after a few minutes
- Computer going to sleep during the workday
- Missing important messages due to status changes

## Installation

1. Download the latest release from the [Releases](https://github.com/TiagoStryke/jbuddy/releases) page
2. Move the JBuddy.app file to your Applications folder
3. Launch JBuddy - it will appear in your menu bar

### First Launch

1. When first launching, right-click (or Control-click) on JBuddy.app and select "Open"
2. Click "Open" in the security dialog
3. JBuddy will appear in your menu bar (top-right of the screen)

### Auto-Start with macOS

To make JBuddy start automatically when you log in:
1. Go to System Settings → General → Login Items
2. Click the "+" button
3. Find and select "JBuddy.app" from Applications
4. Click "Add"

## Usage

- Click the JBuddy icon in the menu bar to access the menu
- Toggle "Enable JBuddy" to turn the service on or off
- Set your working hours in the preferences
- View activity statistics to see how JBuddy has been working for you
- Configure break reminders to help maintain a healthy work routine

### Work Hours

By default, JBuddy operates Monday through Friday, 9 AM to 6 PM. Outside these hours, it enters "After Hours" mode and won't simulate activity, but it remains running to automatically activate the next workday.

## Building from Source

If you want to build JBuddy from source:

1. Clone this repository
2. Create a virtual environment: `python -m venv build_env`
3. Activate the environment: `source build_env/bin/activate`
4. Install requirements: `pip install -r requirements.txt` (or manually install dependencies: rumps, pyobjc)
5. Build the app: `python setup.py py2app`

The built application will be available in the `dist` folder.

## Requirements

- macOS 10.13 or later
- Microsoft Teams application installed

## Version History

### v1.2.0 (April 2025)
- Added activity statistics tracking
- Added break reminders after 1 hour of continuous activity
- Improved persistence across days (now stays running after work hours)
- Fixed issues with Mac sleep prevention

### v1.1.0 (March 2025)
- Added menu bar icons for active and idle states
- Improved mouse movement algorithm
- Added manual toggle for active/idle states
- Fixed bug with work hours detection

### v1.0.0 (February 2025)
- Initial release
- Basic activity simulation
- Work hours configuration
- Menu bar integration

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [rumps](https://github.com/jaredks/rumps) for the macOS menu bar interface
- Uses [py2app](https://github.com/ronaldoussoren/py2app) for macOS application packaging
- Icons created with macOS IconBuild
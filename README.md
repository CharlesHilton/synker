# synker
An application to provide native access to NAS shares.

# Goals
Create an application that can provide native access to common NAS devices via the internet. The application should be able to mount storage to the local device such as Windows, Mac, Linux, or Android device. The host device would need to provide a server that provides access to shares based on the server device's user's authorizations. Files and folders should be able to be selectively synced.

## Current Plan
- Develop a server application for a Western Digital MyCloud PR4100 running MyCloud OS 5.
- Develop a client side application that runs on Windows 11, Linux, Android.
# Twitch-OAuth
A simple application for retrieving your access token and refresh token for your twitch applications

### Requirements

**Crates** :: *"cargo add <crate_name>"*
  - tokio
  - url
  - serde
  - serde_json
  - reqwest
  - open

**Twitch Application CliendID and ClientSecret**
  - Create an application on the [twitch developer site](https://dev.twitch.tv/console)
      - *You will get a client_id and client_secret that you will need* 
  - You also need to set the redirect url in the application to http://localhost:8080/
      - *Remember the '/' at the end of the url; it must match exactly*


### Getting started
  - Run the application once in order to create an empty config.json file.
  - If you are running in debug, the config.json will be created in the root directory of the project not in the /src directory
  - Enter you twitch client_id and client_secret as the appropriate values in the config.json
  - Run the application again. If there is no refresh token the application will open a webbrowser that 

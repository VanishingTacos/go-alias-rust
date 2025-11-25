Build Instructions
1. add the following file
  shortcuts.json
  the content is the following format
  {
    "alas": "url"
  }
2. build rust app: cargo run


USAGE:
/
Mistype any shortcuts to see all your shortcuts

/sql

have button to input a new connection that contains
- Host
- Database name
- user
- password
then
- securely save on encrypted file 
- offer as selectable of all the connections and default to one.

if there is a selected connection it will have a scrollable helper to show what tables are in db then a text input to input your sql to run. then there is a output table display under that. 

then there is a export to csv under that

/note

- if you paste a json or dict it will auto format
- has a basic save button just historically


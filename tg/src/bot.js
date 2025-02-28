const Discord = require('discord.js');

let discordClient;

// Discord Bot Setup
function setupDiscordBot(discordToken, gameContext) {
    discordClient = new Discord.Client();

    discordClient.on('ready', () => {
        console.log(`Logged in as ${discordClient.user.tag}!`);
    });

    discordClient.on('message', msg => {
        if (msg.content.startsWith('!move')) {
            const moveInput = msg.content.substring(6).trim(); // Remove '!move' and trim whitespace
            if (gameContext && gameContext.handleMove) {
                gameContext.handleMove(moveInput, msg.channel);
            } else {
                msg.channel.send('Game context not properly initialized.');
            }
        } else if (msg.content === '!board') {
            if (gameContext && gameContext.displayBoard) {
                gameContext.displayBoard(msg.channel);
            } else {
                msg.channel.send('Game context not properly initialized.');
            }
        } else if (msg.content === '!help') {
            msg.channel.send('Available commands:\n!board - Displays the chessboard\n!move <startCol><startRow> <targetCol><targetRow> - Makes a move (e.g., !move a2 b3)');
        }
    });

    discordClient.login(discordToken);

    discordClient.on("error", console.error);

    console.log('Discord bot is running...');
}

module.exports = {
    setupDiscordBot
};

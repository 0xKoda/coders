const TelegramBot = require('node-telegram-bot-api');

let bot;

function setupBot(token) {
    bot = new TelegramBot(token, { polling: true });

    bot.on('message', (msg) => {
        const chatId = msg.chat.id;
        const messageText = msg.text;

        if (messageText) {
            switch (messageText) {
                case '/hello':
                    bot.sendMessage(chatId, 'Hello there!');
                    break;

                case '/ping':
                    bot.sendMessage(chatId, `Pong!`);
                    break;

                default:
                    bot.sendMessage(chatId, "I don't know that command!");
            }
        }
    });
}

module.exports = {
    setupBot
};
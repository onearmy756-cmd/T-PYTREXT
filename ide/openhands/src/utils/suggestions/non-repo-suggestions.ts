import { I18nKey } from "#/i18n/declaration";
import { MTECHOS_PLATFORM_ACTIONS } from "#/utils/mtechos-platform-actions";

const KEY_1 = I18nKey.SUGGESTIONS$HACKER_NEWS;
const VALUE_1 = `Please write a bash script which displays the top story on Hacker News. It should show the title, the link, and the number of points.
The script should only use tools that are widely available on unix systems, like curl and grep.`;

const KEY_2 = I18nKey.SUGGESTIONS$HELLO_WORLD;
const VALUE_2 = `I want to create a Hello World app in Javascript that:
* Displays Hello World in the middle.
* Has a button that when clicked, changes the greeting with a bouncing animation to fun versions of Hello.
* Has a counter for how many times the button has been clicked.
* Has another button that changes the app's background color.`;

const KEY_3 = I18nKey.SUGGESTIONS$TODO_APP;
const VALUE_3 = `I want to create a VueJS app that allows me to:
* See all the items on my todo list
* add a new item to the list
* mark an item as done
* totally remove an item from the list
* change the text of an item
* set a due date on the item

This should be a client-only app with no backend. The list should persist in localStorage.`;

// MTECHOS quick prompts — vinatoka CHANZO KIMOJA (utils/mtechos-platform-actions.tsx):
// prompts zilezile zinazotumiwa na vitufe vya Home (MtechosQuickActions), hivyo
// mfumo mmoja — kila action inaingia chat ile ile ya home/conversation.
const MTECHOS_FROM_SINGLE_SOURCE: Record<string, string> = Object.fromEntries(
  MTECHOS_PLATFORM_ACTIONS.map((action) => [action.key, action.prompt]),
);

export const NON_REPO_SUGGESTIONS: Record<string, string> = {
  [KEY_1]: VALUE_1,
  [KEY_2]: VALUE_2,
  [KEY_3]: VALUE_3,
  ...MTECHOS_FROM_SINGLE_SOURCE,
};

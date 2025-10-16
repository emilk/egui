## General

- Interact size is great, but it SHOULD or MUST be at least this size ? Maybe add a parameter for this choice ?

## Button

- If hovering a button add a stroke, the ui shift. Maybe add the option to avoid this resize by making the frame smaller ?

## Label

- I understand checking if a sense has been set by the user, but why check if it's different than hover ? Is it for technical purpose or purely to avoid confusion with a link ?

- Selecting the text while being underlined move the underline

## Checkbox

- Checkbox are in fact a label and a check box. To propagate the CheckBoxStyle to the label we need to use a scope or something similar and change the global style. Why bind label to the check box, the user can add it himself (To keep the great prototyping speed we could keep this behavior in a Ui method, that would just merge)

- Propose different type of checkmark ? (eg. small square, dot, filled, custom, etc)

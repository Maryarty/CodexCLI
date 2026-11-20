"""A long transcript answer with a width-sensitive list and a short list."""

from scenario import Scenario, Turn, chunks, text_response

TEXT = """### Результат

Один [сценарий](https://example.com/compare) для двух размеров терминала.

#### В каком порядке строить

- **Первый шаг.** Берём существующий индекс, добавляем точные ориентиры и проверяем, в какие файлы нужно внести изменения.
- **Второй шаг.** Расширяем поиск.
- **Третий шаг.** Сравниваем итог.

#### Проверки

- Старт
- Финиш

END-list-layout
"""
SCENARIO = Scenario(
    name="markdown-list-layout",
    description="completed assistant lists reflow to the available width",
    turns=[
        Turn(
            "Show the steps.",
            [text_response("list-layout", chunks(TEXT, 16), pace=0.04)],
        ),
    ],
    sentinel="END-list-layout",
)

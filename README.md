# Bad Words

## Оглавление

1. [Описание](#описание)
2. [Установка](#установка)
   - [Требования](#требования)
   - [Установка с GitHub](#github)
3. [Использование](#использование)
   - [Инициализация](#инициализация)
   - [Примеры использования](#примеры-использования)
   - [Методы](#методы)
     - [`initialize_language_files()`](#initialize_language_files)
     - [`initialize_bad_words()`](#initialize_bad_words)
     - [`compile_patterns()`](#compile_patterns)
     - [`add_words()`](#add_words)
     - [`similar()`](#similar)
     - [`filter_text()`](#filter_text)
     - [`get_all_languages()`](#get_all_languages)
4. [Поддерживаемые языки](#поддерживаемые-языки)
5. [Полный пример использования](#полный-пример-использования)

## Описание

`BadWords` - это библиотека для фильтрации нецензурной лексики из текста. Она поддерживает различные языки и позволяет добавлять пользовательские слова.

## Установка

### Требования

- Python 3.6 и выше

### GitHub

```bash
pip3 install git+https://github.com/FlacSy/badwords.git
```

## Использование

### Инициализация

```python
ProfanityFilter(languages: List[str] = None, all_languages: bool = False)
```

#### Параметры

- `languages` (список строк, необязательно): Список языков, для которых будут загружены слова нецензурной лексики. Если не указано, будут использованы все доступные языки при установке флага `all_languages` в `True`.
- `all_languages` (логическое значение, необязательно): Флаг для загрузки слов нецензурной лексики для всех доступных языков. По умолчанию `False`.

### Примеры использования

```python
from badwords import ProfanityFilter

# Инициализация с использованием английского и испанского языков
my_filter = ProfanityFilter(languages=['en', 'sp'])

# Инициализация с использованием всех доступных языков
my_filter_all = ProfanityFilter(all_languages=True)
```

### Методы

#### `initialize_language_files()`

Инициализация файлов языков.

##### Возвращаемое значение

- Словарь, который сопоставляет имена языков с путями к файлам.

##### Пример

```python
language_files = my_filter.initialize_language_files()
print(language_files)
```

#### `initialize_bad_words()`

Инициализация слов нецензурной лексики для каждого языка.

##### Возвращаемое значение

- Словарь, который сопоставляет имена языков с наборами слов нецензурной лексики.

##### Пример

```python
bad_words = my_filter.initialize_bad_words()
print(bad_words)
```

#### `compile_patterns()`

Компиляция регулярных выражений для слов нецензурной лексики.

##### Возвращаемое значение

- Словарь, который сопоставляет имена языков с компилированными регулярными выражениями.

##### Пример

```python
patterns = my_filter.compile_patterns()
print(patterns)
```

#### `add_words(words: List[str])`

Добавление пользовательских слов нецензурной лексики в фильтр.

##### Параметры

- `words` (список строк): Список пользовательских слов нецензурной лексики.

##### Пример

```python
my_filter.add_words(['customword1', 'customword2'])
```

#### `similar(a: str, b: str)`

Вычисление коэффициента сходства между двумя строками.

##### Параметры

- `a` (строка): Первая строка.
- `b` (строка): Вторая строка.

##### Возвращаемое значение

- Коэффициент сходства (дробное число).

#### `filter_text(text: str, match_threshold: float = 0.8, replace_character=None)`

Проверка, содержит ли заданный текст нецензурную лексику.

##### Параметры

- `text` (строка): Входной текст для проверки.
- `match_threshold` (дробное число, необязательно): Порог для совпадения по схожести. По умолчанию `0.8`.
- `replace_character` (символ или None, необязательно): Символ для замены непристойных слов. Если None, возвращает True/False. По умолчанию `None`.

##### Возвращаемое значение

- `True` если найдена нецензурная лексика, `False` в противном случае. Если `replace_character` указан, возвращает отфильтрованный текст.

##### Пример

```python
# Проверка на наличие нецензурной лексики
contains_profanity = my_filter.filter_text('This is some bad text', match_threshold=0.9)
print(contains_profanity)  # True или False

# Проверка на наличие нецензурной лексики с заменой
filtered_text = my_filter.filter_text('This is some bad text', replace_character='*')
print(filtered_text)  # Текст с заменёнными непристойными словами
```

### `get_all_languages()`

Получение списка всех доступных языков.

#### Возвращаемое значение

- Список строк, содержащий коды всех поддерживаемых языков.

##### Пример

```python
all_languages = my_filter.get_all_languages()
print(all_languages)  # ['en', 'sp', 'fr', 'de', ...]
```

## Поддерживаемые языки

В настоящее время `BadWords` поддерживает 26 языков:

- `br` - Португальский (Бразилия)
- `cz` - Чешский
- `da` - Датский
- `de` - Немецкий
- `du` - Голландский
- `en` - Английский
- `fi` - Финский
- `fr` - Французский
- `gr` - Греческий
- `hu` - Венгерский
- `in` - Индонезийский
- `it` - Итальянский
- `ja` - Японский
- `ko` - Корейский
- `lt` - Литовский
- `no` - Норвежский
- `pl` - Польский
- `po` - Португальский (Европейский)
- `ro` - Румынский
- `ru` - Русский
- `sp` - Испанский
- `sw` - Шведский
- `th` - Тайский
- `tu` - Турецкий
- `ua` - Украинский

## Полный пример использования

```python
from badwords import ProfanityFilter

# Создаем экземпляр фильтра, указывая нужные языки
my_filter = ProfanityFilter(languages=['en', 'sp'])

# Функция для проверки текста на наличие нецензурной лексики
def check_profanity(text: str):
    result = my_filter.filter_text(
        text=text,
        match_threshold=0.9
    )

    if result:
        print("Этот текст содержит нецензурную лексику.")
    else:
        print("Этот текст не содержит нецензурной лексики.")

# Функция для проверки текста на наличие нецензурной лексики с заменой
def check_profanity_with_replace(text: str):
    result = my_filter.filter_text(
        text=text,
        match_threshold=0.8,
        replace_character="*"
    )

    print(result)

if __name__ == "__main__":
    check_profanity("Text with inappropriate words")
    check_profanity_with_replace("Text with inappropriate words")
```
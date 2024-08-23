---
title: У base64 есть неподвижная точка
time: August 3, 2024
discussion: https://t.me/alisa_rummages/146
intro: |
    ```shell
    $ </dev/urandom base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 \
         | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 \
         | base64 | head -1
    Vm0wd2QyUXlVWGxWV0d4V1YwZDRWMVl3WkRSV01WbDNXa1JTVjAxV2JETlhhMUpUVmpBeFYySkVU

    $ </dev/urandom base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 \
         | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 \
         | base64 | head -1
    Vm0wd2QyUXlVWGxWV0d4V1YwZDRWMVl3WkRSV01WbDNXa1JTVjAxV2JETlhhMUpUVmpBeFYySkVU
    ```
---

*Пост написан по мотивам [давнего треда на Reddit](https://www.reddit.com/r/compsci/comments/18234a/the_base64_encoder_has_a_fixed_point/).*

```shell
$ </dev/urandom base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 \
     | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 \
     | base64 | head -1
Vm0wd2QyUXlVWGxWV0d4V1YwZDRWMVl3WkRSV01WbDNXa1JTVjAxV2JETlhhMUpUVmpBeFYySkVU

$ </dev/urandom base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 \
     | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 | base64 \
     | base64 | head -1
Vm0wd2QyUXlVWGxWV0d4V1YwZDRWMVl3WkRSV01WbDNXa1JTVjAxV2JETlhhMUpUVmpBeFYySkVU
```

### Завязка

Мне нравятся p-адики. Я их не понимаю и никогда особо не изучала, матанализ я знаю разве что на уровне действительных/комплексных чисел. Но, как в меме с distracted boyfriend, я не могу их не уважать: по сравнению со сложностью представления действительных чисел, p-адики просто рай.

Вот, например, многие непрерывные функции обладают таким свойством. Если длина общего префикса чисел $a$ и $b$ стремится к бесконечности, то и длина общего префикса $f(a)$ и $f(b)$ стремится к бесконечности. Это не всегда так: например, для $f(x) = x + 1 - \sqrt{2}$ для $a$, стремящегося к $\sqrt{2}$ снизу, и $b$, стремящегося к $\sqrt{2}$ сверху, условие выполняться будет, а следствие -- нет.

На практике такая интерпретация все равно полезна. Во-первых, с ее помощью можно быстро проверять гипотезы и на глаз что-то оценивать. А во-вторых, эвристикой она является только для действительных чисел, а вот в p-адиках она берется за определение.

Похожим образом можно смотреть на сужающие отображения. Для большинства аргументов $f(a)$ и $f(b)$ имеют более длинный общий префикс, чем $a$ и $b$? Вероятно, отображение сужающее. Ну, как вероятно -- в p-адиках это тоже определение.

Короче говоря, p-адики -- это такой промежуточный мир между числами и строками. Поэтому, когда я вижу какие-то строки и отображения на строках, которые выглядят как-то интересно и как будто бы сходятся, я радуюсь, что я не одинока и что есть теория, которая примерно таким и занимается. Далеко не всегда ее методы применимы в общем случае, но по крайней мере идеи почерпать обычно можно.


### base64

В программировании сужающие отображения встречаются на удивление часто, только мы их называем encoding'ами. Base64, например, относится к этой категории: первые $n$ бит входа однозначно определяют как минимум первые $\lfloor n / 6 \rfloor \cdot 8$ бит выхода, и при $n \ge 18$ второе всегда больше первого. Получается, если вы возьмете 18-битное число, дополните его мусором и начнете итеративно применять к нему base64, на каждом шаге вы будете получать все больше и больше фиксированных бит.

При этом, поскольку первый бит выхода всегда `0`, получается такая картина: если начать со строки с нулем известных бит и применять к ней base64 итеративно, то сначала мы получим 1 известный бит, потому случится что-то нам непонятное, а потом сработает правило $n \ge 18$, и число известных бит будет опять точно расти. Возникает вопрос: можно ли эту "дырку" между $n = 1$ и $n = 18$ заклеить? Да, можно, если отслеживать не только количество известных бит, но и интервал их значений. К сожалению, это требует опоры на конкретный алфавит base64. Руками это делать неудобно и муторно, обойдемся питоном:

```python expansible
ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"

studied_bits = 0
bit_range_min = ""
bit_range_max = ""

for _ in range(8):
    next_studied_bits = 0
    next_bit_range_min = ""
    next_bit_range_max = ""
    had_non_equal_before = False
    for offset in range(0, studied_bits + 1, 6):
        min_sixlet = int(bit_range_min[offset:offset + 6].ljust(6, "0"), 2)
        max_sixlet = int(bit_range_max[offset:offset + 6].ljust(6, "1"), 2)
        mid_max_sixlet = 63 if had_non_equal_before else max_sixlet
        mid_min_sixlet = 0 if had_non_equal_before else min_sixlet
        min_octet = bin(min(map(ord, ALPHABET[min_sixlet:mid_max_sixlet + 1])))[2:].rjust(8, "0")
        max_octet = bin(max(map(ord, ALPHABET[mid_min_sixlet:max_sixlet + 1])))[2:].rjust(8, "0")
        next_studied_bits += 6
        next_bit_range_min += min_octet
        next_bit_range_max += max_octet
        if min_sixlet != max_sixlet:
            had_non_equal_before = True

    studied_bits = next_studied_bits
    bit_range_min = next_bit_range_min
    bit_range_max = next_bit_range_max

    print("Bits studied:", studied_bits)
    print("Range:", bit_range_min, "..=", bit_range_max)
    prefix = 0
    while prefix < studied_bits and bit_range_min[prefix] == bit_range_max[prefix]:
        prefix += 1
    print("Common prefix:", bit_range_min[:prefix], "length", prefix)
    print()
```

```text expansible
Bits studied: 6
Range: 00101011 ..= 01111010
Common prefix: 0 length 1

Bits studied: 12
Range: 0100101100101011 ..= 0110010101110110
Common prefix: 01 length 2

Bits studied: 18
Range: 010100110010101100101011 ..= 010110100101100001100010
Common prefix: 0101 length 4

Bits studied: 24
Range: 01010101001010110010101100101011 ..= 01010111011011000110100001101001
Common prefix: 010101 length 6

Bits studied: 30
Range: 0101011000110000001010110010101100101011 ..= 0101011001111010011110000110111101100001
Common prefix: 010101100 length 9

Bits studied: 36
Range: 010101100110101000101011001010110010101100101011 ..= 010101100110111001110000011110100110001001111010
Common prefix: 0101011001101 length 13

Bits studied: 42
Range: 01010110011011010011000000101011001010110010101100101011 ..= 01010110011011010111101001110111011001010110110101001010
Common prefix: 01010110011011010 length 17

Bits studied: 48
Range: 0101011001101101001100000010101100101011001010110010101100101011 ..= 0101011001101101001100010111101001100100011110100101011001110100
Common prefix: 01010110011011010011000 length 23
```

Итак, любая строка после восьмикратного применения к ней base64 будет начинаться с фиксированного префикса `01010110011011010011000` (`Vm?`, где `?` -- `0` или `1`). Его длина достаточно большая для того, чтобы применить лемму о $n \ge 18$ и сделать вывод, что каждый префикс $base64^n(s)$ при достаточно большом $n$ не зависит от $s$, причем длина этого префикса растет экспоненциально. Сильнее, чем так, сузиться сложно.


### Пределы

Пусть от $s$ префикс не зависит, но как он зависит от $n$? Вывод скрипта намекает, что есть некоторая одна длинная строка, к которой все стремится:
```
0
01
0101
010101
010101100
0101011001101
01010110011011010
01010110011011010011000
```

Докажем это. Обозначим за $p(s)$ максимальный гарантированный общий префикс среди всех строк $base64(s || t)$. Обратите внимание, что это не то же самое, что операция перехода от одной строки из таблицы выше к следующей: таблица учитывала более строгие ограничения на исследуемые строки, чем префиксность. Например, $p(0) = 0$, а не `01`. Впрочем, `p(01010110011011010) = 01010110011011010011000` все еще верно, и этого нам хватит.

Доказательство будет по индукции. Пусть $s$ -- префикс $p(s)$. Тогда, раз $p(s)$ начинается с $s$, то и $p(p(s))$ начинается с $p(s)$. $p(s)$ по определению является префиксом в частности $base64(p(s))$ (ведь $p(s)$ начинается с $s$). Аналогично, $p(p(s))$ является префиксом в частности $base64(p(s))$ (ведь $p(s)$ начинается с $p(s)$). Но более короткий префикс строки обязательно должен быть префиксом более длинного префикса той же строки, то есть $p(s)$ -- префикс $p(p(s))$. Этот переход заканчивает доказательство.


### Свойства

Теперь мы знаем, что предел существует и единственен. Как же он выглядит целиком?

```python
import base64

n = 23
s = b"Vm0"

for _ in range(20):
    n = n // 6 * 8
    s = base64.b64encode(s)[:(n + 7) // 8]

print(s[:n // 8].decode() + "...")
```

```
Vm0wd2QyUXlVWGxWV0d4V1YwZDRWMVl3WkRSV01WbDNXa1JTVjAxV2JETlhhMUpUVmpBeFYySkVUbGhoTVVwVVZtcEJlRll5U2tW
VWJHaG9UVlZ3VlZadGNFSmxSbGw1VTJ0V1ZXSkhhRzlVVmxaM1ZsWmFkR05GU214U2JHdzFWVEowVjFaWFNraGhSemxWVm14YU0x
WnNXbUZrUjA1R1UyMTRVMkpIZHpGV1ZFb3dWakZhV0ZOcmFHaFNlbXhXVm0xNFlVMHhXbk5YYlVaclVqQTFSMVV5TVRSVk1rcEla
SHBHVjFaRmIzZFdha1poVjBaT2NtRkhhRk5sYlhoWFZtMHhORmxWTUhoWGJrNVlZbFZhY2xWcVFURlNNVlY1VFZSU1ZrMXJjRWxh
U0hCSFZqRmFSbUl6WkZkaGExcG9WakJhVDJOdFJraGhSazVzWWxob1dGWnRNSGhPUm14V1RVaG9XR0pyTlZsWmJGWmhZ...
```

Выглядит... случайно, как минимум ациклично. Это можно доказать в три шага.

Предположим, что эта предельная строка $s$ "рациональная", т.е. зацикливается с периодом ровно $k$ с индекса $8nk$ (или раньше). Тогда она циклится и с индекса $6nk$ (ведь при раскодировании циклической строки получается циклическая) с тем же периодом $k$ (ведь период строки не зависит от того, с какого места считать). Следовательно, $s[8nk:] = base64(s[6nk:]) = base64(s[8nk:])$, то есть $s[8nk:]$ -- неподвижная точка base64; но такая точка одна, сама $s$. Значит, $s$ на самом деле не просто "рациональная", а циклическая строка.

Оценим теперь период $s$. Если $k$ -- минимальный период, то $8k/(8,k)$ -- также период. Поскольку $s = s[8k/(8,k):]$, $s = base64^{-1}(s) = base64^{-1}(s[8k/(8,k):]) = s[6k/(8,k):]$, т.е. $6k/(8,k)$ -- период. Любой период делится на минимальный период, поэтому в частности $k | 6k/(8,k)$, откуда $(8,k) | 6$, или, иными словами, $k$ не кратно $4$.

Для дальнейшего перехода придется воспользоваться свойствами конкретного алфавита base64. Раз $s$ начинается с `0101011001101101`, то и $s[k:]$ начинается с `0101011001101101`. Разобьем эту строку по октетам. В зависимости от $k \bmod 8$ это разбиение может выглядеть одним из следующих способов:
- $k \bmod 8 = 0$ -- невозможно по предыдущему параграфу
- $k \bmod 8 = 1$ -- `?0101011 00110110 1???????`
- $k \bmod 8 = 2$ -- `??010101 10011011 01??????`
- $k \bmod 8 = 3$ -- `???01010 11001101 101?????`
- $k \bmod 8 = 4$ -- невозможно по предыдущему параграфу
- $k \bmod 8 = 5$ -- `?????010 10110011 01101???`
- $k \bmod 8 = 6$ -- `??????01 01011001 101101??`
- $k \bmod 8 = 7$ -- `???????0 10101100 1101101?`

В каждом из этих вариантов обязательно найдется октет с единицей в старшем бите, а в выводе base64 такого не бывает. Противоречие.


### Генерация

Ацикличные последовательности помимо математических свойств интересны тем, что генерировать их с конечным объемом памяти в RAM-модели невозможно. С $O(n)$ памяти генерировать мы уже умеем, для этого много ума не надо: бери да итерируй. Можно ли лучше?

Да, можно. Предложим алгоритм, возвращающий по числу $n$ значения битов на позициях $n, n+1, \dots, n+23$. (Почему так много сейчас станет понятно.) Эти $24$ бита каким-то образом содержатся в октетах с индексами границ, кратными $8$. В общем случае это будет четыре октета на некоторых позициях $[8k; 8k+32)$, которые однозначно восстанавливаются из четырех сексетов на позициях $[6k; 6k+24)$. А для того, чтобы узнать эти $24$ бита, достаточно сделать рекурсивный вызов к тому же алгоритму. Осталось не забыть про базу рекурсии $n = 0$ с захардкоженным значением `010101100110110100110000`. На один такой запрос уходит $O(\log n)$ времени и столько же памяти. Суммарно для генерации строки длины $n$ понадобится $O(n \log n)$ времени и $O(\log n)$ памяти.

Напоследок статистическое свойство. В $492$-символьной строке из примера выше символ `V` встречается $45$ раз, а `f` не встречается ни разу. Почему? base64 переводит `010101` в `01010110` (`V`), по сути размножая ее на каждом шагу. А вот `f` получается из `011111`, который получиться может только из октетов `011111??`, `??????01 1111????`, `????0111 11??????`, `??011111`; ни один из вариантов не состоит исключительно из символов из алфавита base64, т.е. `f` не может появиться вот вообще никак.

Короче говоря, использовать эту строку как источник рандома не стоит. Но если хочется полюбоваться, вот ✨ ОНА ✨:

---

```
Vm0wd2QyUXlVWGxWV0d4V1YwZDRWMVl3WkRSV01WbDNXa1JTVjAxV2JETlhhMUpUVmpBeFYySkVUbGhoTVVwVVZtcEJlRll5U2tW
VWJHaG9UVlZ3VlZadGNFSmxSbGw1VTJ0V1ZXSkhhRzlVVmxaM1ZsWmFkR05GU214U2JHdzFWVEowVjFaWFNraGhSemxWVm14YU0x
WnNXbUZrUjA1R1UyMTRVMkpIZHpGV1ZFb3dWakZhV0ZOcmFHaFNlbXhXVm0xNFlVMHhXbk5YYlVaclVqQTFSMVV5TVRSVk1rcEla
SHBHVjFaRmIzZFdha1poVjBaT2NtRkhhRk5sYlhoWFZtMHhORmxWTUhoWGJrNVlZbFZhY2xWcVFURlNNVlY1VFZSU1ZrMXJjRWxh
U0hCSFZqRmFSbUl6WkZkaGExcG9WakJhVDJOdFJraGhSazVzWWxob1dGWnRNSGhPUm14V1RVaG9XR0pyTlZsWmJGWmhZMnhXY1ZG
VVJsTk5WbFkxVkZaU1UxWnJNWEpqUld4aFUwaENTRlpxUm1GU2JVbDZXa1prYUdFeGNHOVdha0poVkRKT2RGSnJhR2hTYXpWeldX
eG9iMWRHV25STlNHaFBVbTE0VjFSVmFHOVhSMHB5VGxac1dtSkdXbWhaTW5oWFkxWkdWVkpzVGs1V2JGa3hWa1phVTFVeFduSk5X
RXBxVWxkNGFGVXdhRU5UUmxweFVtMUdVMkpWYkRaWGExcHJZVWRGZUdOSE9WZGhhMHBvVmtSS1QyUkdTbkpoUjJoVFlYcFdlbGRY
ZUc5aU1XUkhWMjVTVGxOSGFGQlZiVEUwVmpGU1ZtRkhPVmhTTUhCNVZHeGFjMWR0U2tkWGJXaGFUVzVvV0ZreFdrZFdWa3B6Vkdz
MVYySkdhM2hXYTFwaFZURlZlRmR1U2s1WFJYQnhWV3hrTkdGR1ZYZGhSVTVVVW14d2VGVnRNVWRWTWtwV1lrUmFXR0V4Y0hKWlZX
UkdaVWRPU0U5V1pHaGhNSEJ2Vm10U1MxUXlVa2RUYmtwb1VqSm9WRmxZY0ZkbGJHUllaVWM1YVUxWFVraFdNalZUVkd4T1NHRkdR
bFppVkVVd1ZtcEdVMVp0UmtoUFZtaFRUVWhDTlZaSGVHRmpNV1IwVTJ0a1dHSlhhR0ZVVnpWdlYwWnJlRmRyWkZkV2EzQjZWa2R6
TVZZeVNrZGhNMmhYWVRGd2FGWlVSbFpsUm1SMVUyczFXRkpZUW5oV1YzaHJUa2RHUjFaWVpHaFNWVFZWVlcxNGQyVkdWblJOVldS
V1RXdHdWMWxyVW1GWFIwVjRZMGhLV2xaWFVrZGFWV1JQVTBVNVYxcEhhR2hOU0VKMlZtMTBVMU14VVhsVmEyUlVZbXR3YjFWcVNt
OVdSbXhaWTBaa2JHSkhVbGxhVldNMVlWVXhXRlZyYUZkTmFsWlVWa2Q0VDFOSFJrZFJiRnBwVmtWVmQxWnRjRWRWTVZwMFVtdG9V
Rlp0YUZSVVZXaERUbFphU0dWSFJtcE5WMUl3VlRKMGExZEhTbGhoUjBaVlZucFdkbFl3V25KbFJtUnlXa1prVjJFelFqWldhMlI2
VFZaWmVWTnJaR2hOTW1oWVdWUkdkMkZHV2xWU2JGcHNVbTFTTVZVeWN6RlhSa3BaVVc1b1YxWXphSEpVYTJSSFVqRmFXVnBIYUZO
V1ZGWldWbGN4TkdReVZrZFdibEpPVmxkU1YxUlhkSGRXTVd4eVZXMUdXRkl3VmpSWk1HaExWMnhhV0ZWclpHRldWMUpRVlRCVk5W
WXhjRWhoUjJoT1UwVktNbFp0TVRCVk1VMTRWVmhzVm1FeVVsVlpiWFIzWWpGV2NWTnRPVmRTYlhoYVdUQmFhMkpIU2toVmJHeGhW
bGROTVZsV1ZYaFhSbFp5WVVaa1RtRnNXbFZXYTJRMFZERk9TRkpyWkZKaVJuQndWbXRXVm1ReFduUmpSV1JXVFZad01GVnRkRzlW
UmxwMFlVWlNWVlpYYUVSVWJGcGhVMGRXU0ZKdGNFNVdNVWwzVmxSS01HRXhaRWhUYkdob1VqQmFWbFp1Y0Zka2JGbDNWMjVLYkZK
dFVubFhhMXByVmpKRmVsRnFXbGRoTWxJMlZGWmFXbVZXVG5KYVIyaE9UVzFvV1ZkV1VrZGtNa1pIVjJ4V1UySkdjSE5WYlRGVFRW
WlZlV042UmxoU2EzQmFWVmMxYjFZeFdYcGhTRXBWWVRKU1NGVnFSbUZYVm5CSVlVWk9WMVpHV2xkV2JHTjRUa2RSZVZaclpGZGli
RXBQVm14a1UxWXhVbGhrU0dSWFRWZDRlVlpYTVVkWFJrbDNWbXBTV2sxSGFFeFdNbmhoVjBaV2NscEhSbGRXTVVwUlZsUkNWazVX
V1hoalJXaG9VakpvVDFVd1ZrdE5iRnAwVFZSQ1ZrMVZNVFJXVm1oelZtMUZlVlZzVmxwaVdGSXpXV3BHVjJOV1RuUlBWbVJUWWxo
b1lWZFVRbUZoTWtwSVUydG9WbUpIZUdoV2JHUk9UVlpzVjFaWWFGaFNiRnA1V1ZWYWExUnRSbk5YYkZaWFlUSlJNRlpFUms5VFJr
cHlXa1pLYVZKdVFuZFdiWFJYVm0xUmVGZHVVbXBTVjFKWFZGWmFkMDFHVm5Sa1J6bFdVbXh3TUZsVldsTldWbHBZWVVWU1ZXSkdj
R2hWTUdSWFUwWktkR05GTlZkTlZXd3pWbXhTUzAxSFJYaGFSV2hVWWtkb2IxVnFRbUZXYkZwMVkwWmthMkpHYkROV01qVkxZa1pL
ZEZWdWJGaGhNWEJ5Vm1wS1JtVnNSbkZYYkdSb1RXeEpNbFpHV21GWGJWWlhWRzVLWVZJeWFFOVVWekZ2VjFaa1YxVnJaR3ROYTFw
SVZqSjRWMVV5U2tkalNFNVdZbFJHVkZSV1dsWmxWMDQyVW14b1UyRXpRbUZXVm1NeFlqRlplRmRZY0doVFJYQldXVlJLVTFOR1Zu
RlNiVVpZVm01Q1NWbFZXazlXTVZwSFYyeGtWMkpIVGpSVWEyUlNaVlphY2xwR1pHbGlSWEJRVm0xNGExVXhXWGhWYkdoclUwZFNX
RlJXWkRSbFZscFlUVlZrV0ZKcmJETldiWEJUVjJzeFNHRkZlRmROYm1ob1ZqQmFWMk5zY0VoU2JHUlhUVlZ3VWxac1VrTldhelZY
VjFob2FsSlhhRzlWYWtwdlZERlZkMVpyZEU1aVJuQXdWRlpTUTFack1WWk5WRkpYVm0xb2VsWnRNVVpsVmxaelZteHdhVmRHU1hw
WFYzQkhWakpPVjFSdVVsQldiVkpVV1d4b2IxbFdaRlZSYlVab1RXdHdTVlV5ZEc5V2JVcElaVWRvVjJKSFVrOVVWbHB6VmpGYVdX
RkdhRk5pUm5BMVYxWldZV0V4VW5SU2JrNVlZa1phV0ZsVVNsSk5SbHBGVW1zNVZGSnJjSGxYYTFwTFlWWktkVkZ1WkZkaVdGSllW
bTB4VW1WR1pIVlZiWEJUVmpGS1dGWkdXbUZrTURGSFZtNVNhMUo2YkZkVmJYaDNUVVpzVmxkc1RsZFdiSEJaV1ZWV1UxWlhTa2Rq
UjJoV1RVZFNXRlV3V2t0a1IwNUdUbFprVGxaWGQzcFdiWGhUVXpBeFNGSllhR0ZTVjJoVldXdGtiMkl4Vm5GUmJVWlhZa1p3TVZr
d1dtdGhNa3BIWWtST1YwMXFWa3haYTFwTFpFWldkV0pHYUdoTldFSjVWbTF3UzFKdFZuTlNia1pZWWtkU2IxUlhlRXBOYkZwSFYy
MUdXR0pXV2xoV1J6VkxXVlpKZVdGRk9WVldla1oyVmpGYWExWXhWbkphUjNST1lURndTVlpxU2pSV01WVjVVMnRrYWxORk5WZFpi
RkpIVmtaU1YxZHNXbXhXTURReVZXMTRiMVV5UlhwUmJVWlhWbTFOZUZscVJscGxSbVJaWTBkb1ZGSllRbGRYVmxKTFZURk9SMVp1
UmxOaVZWcFpWbTAxUTFOV2JGWlhhemxYVFZad1NGWXllR3RXTWtwSVZHcFNWV0V5VWxOYVZscGhZMnh3UjFwSGJHbFNXR...
```

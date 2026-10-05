import unittest

from inventory import find, in_stock, make_item


class InventoryTest(unittest.TestCase):
    def test_in_stock_drops_empty_items(self):
        items = [make_item("a", 1.0, 0), make_item("b", 2.0, 3)]
        self.assertEqual([item["name"] for item in in_stock(items)], ["b"])

    def test_find_returns_none_for_a_missing_name(self):
        self.assertIsNone(find([], "a"))


if __name__ == "__main__":
    unittest.main()
